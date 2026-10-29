//! sapphire-sync's app server: what `sapphire-sync serve` runs, in either
//! edition. A library, not a binary of its own: the server is set up with the
//! same `sapphire-sync` command that runs it (`sapphire-sync-cli`), so there is
//! no separate server command to tell from the client's.
//!
//! Two builders, bridge-less for one-shot verbs and sync-mounted for serve,
//! because the bridge connection belongs to the serve path only
//! (the review of `0acb194` ruled the one-shot verbs bridge-less):
//!
//! - [`build_oneshot`] hands every one-shot framework verb that needs a server
//!   (`service`, `update`) a bare [`AppServer`] — `dispatch` needs only the app
//!   name and the service spec, and a CLI invocation must not open a bridge or
//!   fail with a bridge error.
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
//!   stops the start. `serve` calls it once per round of
//!   `serve_restarting`, so a `server.restart` gets a fresh runtime.
//!
//! The IPC endpoint, the stop channel and the signals are `run()`'s business
//! in both paths; the app installs nothing there.
//!
//! `ManagedBy` is the transport's, which `SyncRuntime` takes.

use std::path::Path;
use std::sync::Arc;

use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::ipc::ManagedBy;
use sapphire_sync_core::framework::server::{AppServer, BridgeConnector, Settings, SyncRuntime};

mod status;

pub use status::status_rows;

/// The version the server reports, and the command line's clients name: the
/// editions and this library share it (release-plz.toml).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The app's server for one-shot verbs: an `AppServer` with nothing mounted.
///
/// Every [`FrameworkCommand`] except `Serve` dispatches with this as its
/// builder; the framework calls it only for the verbs that read the server
/// (`service_spec()`, what a service manager starts). The others connect to a
/// *running* server over IPC themselves. The app has no settings of its own,
/// so `_settings` holds the framework's alone and nothing here reads them.
///
/// [`FrameworkCommand`]: sapphire_sync_core::framework::server::FrameworkCommand
pub fn build_oneshot(_settings: &Settings) -> AppServer {
    AppServer::new(&CTX, VERSION)
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
///
/// A new runtime per call: the framework's `serve_restarting` builds a server
/// per round, and a server stops the runtime it was given when it stops. `exe`
/// is this executable, looked up once by the caller.
pub fn build_serve(_settings: &Settings, exe: &Path) -> AppServer {
    let runtime = Arc::new(SyncRuntime::connecting(
        &CTX,
        BridgeConnector::standard(CTX.app_name, VERSION),
        exe.to_path_buf(),
        ManagedBy::Service,
    ));
    AppServer::new(&CTX, VERSION)
        .app_api(sapphire_sync_api::API_VERSION)
        .sync(Arc::clone(&runtime))
        .status_rows(status_rows(runtime))
}
