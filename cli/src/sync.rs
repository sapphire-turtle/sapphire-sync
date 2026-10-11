//! The app's own `sync` verb — `sync disable` — and the status rows the app's
//! server reports.
//!
//! Enabling, mapping and listing are the framework's `workspace` verbs; only
//! *disabling* is app-owned, and even it is thin: connect to the running
//! server (connect-only, nothing is ever started) and call the framework's
//! `sync.disable` on the workspace it serves. Status is the framework's
//! report; the app only injects rows.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::backend::protocol as proto;
use sapphire_sync_core::framework::ipc::{ClientInfo, Endpoint, connect_or_absent};
use sapphire_sync_core::framework::server::{StatusRow, SyncRuntime, SyncStatus};
use sapphire_sync_core::framework::workspace::Workspace;

use crate::cli::SyncCommand;

/// Run the parsed `sync` verb, returning the process exit code.
///
/// Any error — a workspace that cannot be resolved, a server that answers but
/// fails the call — propagates to `main`, which prints it in the app's own
/// `sapphire-sync: {err}` shape and exits 1. An absent server is not an
/// error: it is the one-line report below.
pub async fn dispatch(command: SyncCommand, root: Option<PathBuf>) -> anyhow::Result<i32> {
    match command.command {
        crate::cli::SyncSubcommand::Disable => disable(root).await,
    }
}

/// `sync disable`: stop syncing the server's workspace.
///
/// A server serves one workspace (the framework's `workspace.current`), and
/// `sync.disable` names none: it stops syncing that one. The current
/// directory plays no part — the verb acts on the server's workspace wherever
/// it is typed. An explicit `--workspace-dir` is only a guard: it must be a
/// workspace root itself (`Workspace::from_root`, no upward walk), and the
/// verb refuses to run when that is not the server's workspace, so a
/// `sync disable --workspace-dir <dir>` never stops another. Connect-only,
/// nothing is ever started.
///
/// An absent server is the framework's one-line report — `no {app} server is
/// running` on stdout, exit 1, the exact line the framework's own one-shot
/// verbs print when nothing is listening (`run_status_into`) — not an error
/// on stderr. Absence is the verb's answer, the same `connect_or_absent`
/// `None` every other one-shot verb reports, and a half-open socket or a
/// shutting-down server folds into it. Every other error — an explicit
/// directory that is not a workspace root, a server with no workspace
/// selected, a workspace mismatch, a probe failure, a failing call —
/// propagates to `main`, which prints it in the app's `sapphire-sync: {err}`
/// shape and exits 1.
///
/// Files and the sync id stay, and success is exit 0 whether or not the
/// workspace was synced at all — the runtime's `disable` is idempotent, so a
/// workspace that was never enabled is a no-op, not an error. Re-enabling via
/// `workspace init --sync` rejoins the same workspace.
async fn disable(root: Option<PathBuf>) -> anyhow::Result<i32> {
    // `from_root` canonicalises the root, so the comparison below is between
    // canonical paths.
    let expected = match root {
        Some(dir) => Some(
            Workspace::from_root(&CTX, &dir)
                .map_err(|e| {
                    anyhow::Error::new(e).context(format!(
                        "{} is not a sapphire-sync workspace root",
                        dir.display()
                    ))
                })?
                .root,
        ),
        None => None,
    };

    let endpoint = Endpoint::for_app(CTX.app_name)?;
    let info = ClientInfo {
        kind: "cli".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        api: proto::API_VERSION,
        app_api: None,
        pid: std::process::id(),
    };
    // Connect-only, and the API gate rides the handshake: a live server of
    // another API version is `ApiVersionMismatch`, which propagates like any
    // other error. `None` is nothing listening — the report, exit 1.
    let Some((client, _)) = connect_or_absent(&endpoint, CTX.app_name, info).await? else {
        println!("no {} server is running", CTX.app_name);
        return Ok(1);
    };
    let current: proto::WorkspaceCurrentResult = client
        .call(proto::WORKSPACE_CURRENT, serde_json::json!({}))
        .await?;
    let Some(current) = current.workspace else {
        anyhow::bail!(
            "the {} server has no workspace selected; run `{} workspace select <dir>`",
            CTX.app_name,
            CTX.app_name
        );
    };
    if let Some(expected) = expected {
        // The server may report its root through another spelling.
        let served = current
            .root
            .canonicalize()
            .unwrap_or_else(|_| current.root.clone());
        if expected != served {
            anyhow::bail!(
                "{} is not the server's workspace ({}); run `{} workspace select` to switch",
                expected.display(),
                current.root.display(),
                CTX.app_name
            );
        }
    }
    let _: proto::Ack = client
        .call(proto::SYNC_DISABLE, serde_json::json!({}))
        .await?;
    println!("sync disabled for {}", current.root.display());
    Ok(0)
}

/// The status rows the app's server reports, for the framework's
/// [`AppServer::status_rows`](sapphire_sync_core::framework::server::AppServer::status_rows).
///
/// The framework's status report renders the app's rows after its own fields,
/// calling this once per report. Every workspace the runtime syncs gets one
/// row — `synced as <id>, N peer(s)`, the workgroup's device count minus this
/// host per the framework's convention, and `paused: <reason>` appended when
/// replication is paused. A workspace that is not synced has no row: the
/// framework's own `workspace show` already reports the server's workspace
/// and whether it syncs.
pub fn status_rows(runtime: Arc<SyncRuntime>) -> Arc<dyn Fn() -> Vec<StatusRow> + Send + Sync> {
    Arc::new(move || {
        // The closure is sync (the framework calls it from a sync context),
        // the runtime's state is async: step onto a blocking thread and poll
        // the calls there. The app's runtimes are always multi-thread —
        // `main`'s and the tests' — where `block_in_place` is legal; on a
        // current-thread runtime it would panic, and the app has none.
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            let mut roots = handle.block_on(runtime.roots());
            // The runtime holds its roots in a map; report them in a stable
            // order.
            roots.sort();
            roots
                .iter()
                .filter_map(|root| {
                    let status = handle.block_on(runtime.status(root));
                    status.enabled.then(|| row_for(root, &status))
                })
                .collect()
        })
    })
}

/// The row one synced workspace renders as.
fn row_for(root: &Path, status: &SyncStatus) -> StatusRow {
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    let id = status
        .workspace_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "-".into());
    let mut value = format!("synced as {id}, {}", peers_text(status.peers));
    if let Some(paused) = &status.paused {
        value.push_str(&format!(", paused: {paused}"));
    }
    StatusRow { name, value }
}

/// `N peer(s)`, singular for one — the framework's own pluralisation.
fn peers_text(peers: usize) -> String {
    format!("{peers} peer{}", if peers == 1 { "" } else { "s" })
}
