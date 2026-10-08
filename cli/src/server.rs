//! The app's servers: bridge-less for one-shot verbs, sync-mounted for serve.
//!
//! Two builders, because the bridge connection belongs to the serve path only
//! (the review of `0acb194` ruled the one-shot verbs bridge-less):
//!
//! - [`build_oneshot`] hands every one-shot framework verb a bare
//!   [`AppServer`] — `dispatch` needs only the app name and the service spec,
//!   and a CLI invocation must not open a bridge or fail with a bridge error.
//!   A `status` on a host without a server answers the framework's "no
//!   sapphire-sync server is running" and exits 1; a `workspace init` likewise
//!   reports the absent server. Neither touches the bridge.
//! - [`build_serve`] additionally connects the bridge — connect-only, nothing
//!   starts one on demand (the framework's global constraint), so `serve` on a
//!   host without a bridge fails with the bridge's own "no sapphire-bridge is
//!   running" message rather than pretending — and mounts the `SyncRuntime`
//!   (the daemon is always service-managed) with [`AppServer::sync`], plus the
//!   app's own status rows with [`AppServer::status_rows`]: one row per synced
//!   workspace, rendered per report from the live runtime.
//!
//! The IPC endpoint, the stop channel and the signals are `run()`'s business
//! in both paths; the app installs nothing there.
//!
//! `BridgeClient` comes from the bridge-api crate: the facade re-exports the
//! bridge *daemon* crate under `framework::bridge`, not `-bridge-api`, so the
//! client is only reachable as a direct dependency (same git source, same
//! branch).

use std::sync::Arc;

use sapphire_bridge_api::{BridgeClient, ManagedBy};
use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::server::{AppServer, SyncRuntime};

use crate::sync;

/// The app's server for one-shot verbs: an `AppServer` with nothing mounted.
///
/// Every [`FrameworkCommand`] except `Serve` dispatches against this: it reads
/// `app_name()` (the endpoint to open) and `service_spec()` (what a service
/// manager starts), and connects to a *running* server over IPC itself. A
/// bridge-less, sync-less server is exactly what the framework's own status
/// tests dispatch against.
pub fn build_oneshot() -> AppServer {
    AppServer::new(&CTX, env!("CARGO_PKG_VERSION"))
}

/// The app's embedded server: bridge connection, sync runtime, `AppServer`.
///
/// Sync is mounted with [`AppServer::sync`]; the runtime is built here because
/// it needs the bridge connection, which is the caller's to open and to close.
/// The app's own status rows are mounted with [`AppServer::status_rows`] —
/// built from the same runtime, so a report reads the live sync state.
pub async fn build_serve() -> anyhow::Result<AppServer> {
    // Connect-only: an absent bridge is the bridge-api's `Error::NotRunning`
    // and the message is exactly the bridge's.
    let bridge = Arc::new(BridgeClient::connect(CTX.app_name, env!("CARGO_PKG_VERSION")).await?);
    let runtime = Arc::new(SyncRuntime::new(
        &CTX,
        bridge,
        std::env::current_exe()?,
        ManagedBy::Service,
    ));
    Ok(AppServer::new(&CTX, env!("CARGO_PKG_VERSION"))
        .sync(Arc::clone(&runtime))
        .status_rows(sync::status_rows(runtime)))
}
