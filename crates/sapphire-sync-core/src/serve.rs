//! The app's server with sync mounted — what `sapphire-sync serve` runs, and
//! what the phone app hosts in its own process.
//!
//! [`build_serve`] mounts a `SyncRuntime` that reaches the bridge itself, plus
//! the app's own status rows ([`status_rows`]): one row per synced workspace,
//! rendered per report from the live runtime. The runtime is connect-only —
//! nothing starts a bridge on demand — and the framework connects when the
//! server runs, and again whenever the bridge restarts. A server whose bridge
//! is not running yet serves without sync until it is; meanwhile `sync.*`
//! answers "not connected to sapphire-bridge".

use std::path::Path;
use std::sync::Arc;

use crate::CTX;
use crate::framework::ipc::ManagedBy;
use crate::framework::server::{AppServer, BridgeConnector, StatusRow, SyncRuntime, SyncStatus};

/// The app's embedded server: sync runtime, `AppServer`.
///
/// The runtime is built here rather than by `AppServer::sync_via_bridge`,
/// which is the same runtime, because the status rows read it too. It is not
/// connected yet: `AppServer::run` makes the first attempt, and keeps it
/// connected from there. The server reports the app's API version
/// (`sapphire-sync-api`) in the handshake, so a client that names another is
/// refused.
pub fn build_serve() -> anyhow::Result<AppServer> {
    let runtime = Arc::new(SyncRuntime::connecting(
        &CTX,
        BridgeConnector::standard(CTX.app_name, env!("CARGO_PKG_VERSION")),
        std::env::current_exe()?,
        ManagedBy::Service,
    ));
    Ok(AppServer::new(&CTX, env!("CARGO_PKG_VERSION"))
        .app_api(sapphire_sync_api::API_VERSION)
        .sync(Arc::clone(&runtime))
        .status_rows(status_rows(runtime)))
}

/// The status rows the app's server reports, for the framework's
/// [`AppServer::status_rows`](crate::framework::server::AppServer::status_rows).
///
/// The framework's status report renders the app's rows after its own fields,
/// calling this once per report. Every workspace the runtime syncs gets one
/// row — `synced as <id>, N peer(s)`, the workgroup's device count minus this
/// host per the framework's convention, and `paused: <reason>` appended when
/// replication is paused. A workspace that is not synced has no row: the
/// framework's own `workspace show` already reports the server's workspace
/// and whether it syncs.
///
/// While the runtime has no connection to the bridge, a first `bridge` row says
/// so: the framework reconnects on its own, and until then nothing syncs.
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
            let bridge = (!runtime.is_connected()).then(|| StatusRow {
                name: "bridge".to_owned(),
                value: "not connected; reconnecting".to_owned(),
            });
            bridge
                .into_iter()
                .chain(roots.iter().filter_map(|root| {
                    let status = handle.block_on(runtime.status(root));
                    status.enabled.then(|| row_for(root, &status))
                }))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn a_runtime_without_its_bridge_says_so_first() {
        // No bridge running: the connector finds none, and the runtime waits.
        let runtime = Arc::new(SyncRuntime::connecting(
            &CTX,
            BridgeConnector::new(|| async { Ok(None) }),
            "/bin/true".into(),
            ManagedBy::Service,
        ));
        runtime.connect().await.unwrap();

        let rows = status_rows(runtime)();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].name, "bridge");
        assert_eq!(rows[0].value, "not connected; reconnecting");
    }
}
