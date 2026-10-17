//! The app's own `sync` verb — `sync disable`. The status rows the app's
//! server reports live in `sapphire_sync_core::serve`, with the server.
//!
//! Enabling, mapping and listing are the framework's `workspace` verbs; only
//! *disabling* is app-owned, and even it is thin: connect to the running
//! server (connect-only, nothing is ever started) and call the framework's
//! `sync.disable` on the workspace it serves. Status is the framework's
//! report; the app only injects rows.

use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::backend::protocol as proto;
use sapphire_sync_core::framework::ipc::{ClientInfo, Endpoint, connect_or_absent};

use crate::cli::SyncCommand;

/// Run the parsed `sync` verb, returning the process exit code.
///
/// Any error — a server that answers but fails the call — propagates to
/// `main`, which prints it in the app's own `sapphire-sync: {err}` shape and
/// exits 1. An absent server is not an error: it is the one-line report below.
pub async fn dispatch(command: SyncCommand) -> anyhow::Result<i32> {
    match command.command {
        crate::cli::SyncSubcommand::Disable => disable().await,
    }
}

/// `sync disable`: stop syncing the server's workspace.
///
/// A server serves one workspace (the framework's `workspace.current`), and
/// `sync.disable` names none: it stops syncing that one. The verb takes no
/// workspace of its own — the current directory plays no part — so it acts
/// on the server's workspace
/// wherever it is typed; `workspace select` is how to point the server at
/// another. Connect-only, nothing is ever started.
///
/// An absent server is the framework's one-line report — `no {app} server is
/// running` on stdout, exit 1, the exact line the framework's own one-shot
/// verbs print when nothing is listening (`run_status_into`) — not an error
/// on stderr. Absence is the verb's answer, the same `connect_or_absent`
/// `None` every other one-shot verb reports, and a half-open socket or a
/// shutting-down server folds into it. Every other error — a server with no
/// workspace selected, a probe failure, a failing call — propagates to
/// `main`, which prints it in the app's `sapphire-sync: {err}` shape and
/// exits 1.
///
/// Files and the sync id stay, and success is exit 0 whether or not the
/// workspace was synced at all — the runtime's `disable` is idempotent, so a
/// workspace that was never enabled is a no-op, not an error. Re-enabling via
/// `workspace init --sync` rejoins the same workspace.
async fn disable() -> anyhow::Result<i32> {
    let endpoint = Endpoint::for_app(CTX.app_name)?;
    let info = ClientInfo {
        kind: "cli".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        api: proto::API_VERSION,
        app_api: Some(sapphire_sync_api::API_VERSION),
        pid: std::process::id(),
    };
    // Connect-only, and the API gates ride the handshake: a live server of
    // another framework or app API version is `ApiVersionMismatch` or
    // `AppApiVersionMismatch`, which propagates like any other error. `None` is nothing listening — the report, exit 1.
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
    let _: proto::Ack = client
        .call(proto::SYNC_DISABLE, serde_json::json!({}))
        .await?;
    println!("sync disabled for {}", current.root.display());
    Ok(0)
}
