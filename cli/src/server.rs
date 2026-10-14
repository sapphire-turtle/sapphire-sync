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
//! - [`build_serve`] additionally mounts a `SyncRuntime` that reaches the
//!   bridge itself (the daemon is always service-managed) with
//!   [`AppServer::sync`], plus the app's own status rows with
//!   [`AppServer::status_rows`]: one row per synced workspace, rendered per
//!   report from the live runtime. The runtime is connect-only — nothing starts
//!   a bridge on demand (the framework's global constraint) — and the framework
//!   connects when the server runs, and again whenever the bridge restarts
//!   (sapphire-framework#244). `serve` on a host whose bridge is not running
//!   yet serves without sync until it is; meanwhile `sync.*` answers "not
//!   connected to sapphire-bridge". A bridge of another API version still
//!   stops the start.
//!
//! The IPC endpoint, the stop channel and the signals are `run()`'s business
//! in both paths; the app installs nothing there.
//!
//! `ManagedBy` is the transport's, which `SyncRuntime` takes.

use std::sync::Arc;

use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::ipc::ManagedBy;
use sapphire_sync_core::framework::server::{AppServer, BridgeConnector, SyncRuntime};

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

/// The app's embedded server: sync runtime, `AppServer`.
///
/// Sync is mounted with [`AppServer::sync`]. The runtime is built here rather
/// than by [`AppServer::sync_via_bridge`], which is the same runtime, because
/// the status rows read it too. It is not connected yet: [`AppServer::run`]
/// makes the first attempt, and keeps it connected from there.
/// The app's own status rows are mounted with [`AppServer::status_rows`] —
/// built from the same runtime, so a report reads the live sync state. The
/// server reports the app's API version (`sapphire-sync-api`) in the
/// handshake, so a client that names another is refused.
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
        .status_rows(sync::status_rows(runtime)))
}
