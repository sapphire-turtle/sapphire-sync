//! The app's server: bridge connection, sync runtime, `AppServer`.
//!
//! `build()` wires the three pieces the framework leaves to the application:
//! the bridge connection (connect-only — a host without a running bridge
//! cannot sync, so `serve` fails with the bridge's own "no sapphire-bridge is
//! running" message rather than pretending), the `SyncRuntime` (the daemon is
//! always service-managed), and the `AppServer` itself. The app's status rows
//! land with Task 3.
//!
//! `BridgeClient` and `ManagedBy` come from the bridge-api crate: the facade
//! re-exports the bridge *daemon* crate under `framework::bridge`, not
//! `-bridge-api`, so the client is only reachable as a direct dependency
//! (same git source, same branch).

use std::sync::Arc;

use sapphire_bridge_api::{BridgeClient, ManagedBy};
use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::server::{AppServer, SyncRuntime};

/// Build this app's server: bridge, sync runtime, and the `AppServer` that
/// owns the workspaces.
///
/// Sync is mounted with [`AppServer::sync`]; the IPC endpoint, the stop
/// channel and the signals are `run()`'s business, so the app installs
/// nothing there.
pub async fn build() -> anyhow::Result<AppServer> {
    // Connect-only: nothing starts a bridge on demand (the framework's global
    // constraint), so an absent bridge is `Error::NotRunning` and the message
    // is exactly the bridge's.
    let bridge = Arc::new(BridgeClient::connect("sapphire-sync", env!("CARGO_PKG_VERSION")).await?);
    let runtime = Arc::new(SyncRuntime::new(
        &CTX,
        bridge,
        std::env::current_exe()?,
        ManagedBy::Service,
    ));
    Ok(AppServer::new(&CTX, env!("CARGO_PKG_VERSION")).sync(runtime))
}

// Reaching `build`'s pieces: `BridgeClient` and `ManagedBy` through the
// bridge-api crate (the facade does not re-export it), `SyncRuntime` through
// the facade's `server` module (the framework's own command layer does the
// same via `sapphire_framework_server`).
