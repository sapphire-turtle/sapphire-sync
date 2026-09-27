//! The serve smoke test: the app's server listens, answers `status`.
//!
//! The two things the framework guarantees the app: a bare invocation *parses*
//! to `Serve`, and `status` against a running in-process server prints
//! `running: true` + pid and exits 0. (The app's own status rows are Task 3.)
//!
//! The server is built from the same pieces `server::build()` wires —
//! `SyncRuntime::new(&CTX, bridge, exe, ManagedBy::Service)` into
//! `AppServer::new(&CTX, VERSION).sync(..)` — except the bridge, which is the
//! `test-util` `StubBridge` (an in-process stand-in: `BridgeClient::connect`
//! needs a running daemon, and starting one is Task 4's e2e harness).

use std::sync::{Arc, Mutex, MutexGuard};

use clap::Parser;
use sapphire_framework_server::sync::testing::StubBridge;
use sapphire_ipc::{Endpoint, probe};
use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::server::AppServer;
use sapphire_sync_core::framework::workspace::AppKind;

// The binary's parse types, compiled into this test crate the same way
// `cli_parse.rs` includes them.
#[path = "../src/cli.rs"]
mod cli;

/// Everything one test needs: the env overrides and their guard.
///
/// The three `SAPPHIRE_SYNC_*` vars route `CTX.init` into the scratch tree,
/// `SAPPHIRE_RUNTIME_DIR` routes the IPC endpoints into it.
struct Env {
    vars: [(&'static str, Option<std::ffi::OsString>); 4],
}

impl Env {
    /// Point the context and the runtime directory at `root`, in one lock.
    fn set(root: &std::path::Path) -> Env {
        let vars = [
            ("SAPPHIRE_SYNC_CACHE_DIR", "cache"),
            ("SAPPHIRE_SYNC_DATA_DIR", "data"),
            ("SAPPHIRE_SYNC_CONFIG_DIR", "config"),
            ("SAPPHIRE_RUNTIME_DIR", "run"),
        ]
        .map(|(name, leaf)| (name, root.join(leaf).display().to_string()));
        // SAFETY: `ENV_LOCK` serialises every read and write of the process
        // environment in this test binary, and the guard holds the lock until
        // `Drop` has restored the previous values — including while unwinding
        // from a panic.
        let previous = unsafe {
            for (name, value) in &vars {
                std::env::set_var(name, value);
            }
            vars.map(|(name, _)| (name, std::env::var_os(name)))
        };
        Env { vars: previous }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        // SAFETY: the same `ENV_LOCK` guard is still held; `Drop` runs before
        // it is released.
        unsafe {
            for (name, value) in &self.vars {
                match value {
                    Some(previous) => std::env::set_var(name, previous),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
}

/// One lock for the whole test binary: env vars are process-global and the
/// test harness runs tests on parallel threads.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Lock the process environment for this test and hold it for the test's body.
///
/// The returned guard is kept alive until the end of the test, so the `Env`
/// restore and every read of these variables are serialised against the other
/// tests of this binary. A poisoned lock only means some other test panicked
/// while holding it; the environment is not invariant-critical here.
fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Wait until something is listening on `endpoint`.
///
/// `tokio::spawn(server.run())` only schedules the server; without this wait a
/// fast first probe would land before `run` has bound the socket, and that one
/// unlucky probe is the whole test failing (a fast probe is not a failure, a
/// deadline is).
async fn wait_until_listening(endpoint: &Endpoint) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    while !probe(endpoint).await.unwrap_or(false) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the server never started listening"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

/// A bare invocation parses to `app: None`, and the default is `Serve`.
#[test]
fn a_bare_invocation_parses_to_serve() {
    let parsed = cli::Cli::try_parse_from(["sapphire-sync"]).unwrap();
    assert!(parsed.app.is_none());
    assert!(matches!(
        cli::FrameworkCommand::default(),
        cli::FrameworkCommand::Serve
    ));
}

/// `serve` against the built server: it listens, and `status` answers.
///
/// The `std` env lock below is deliberately held across the awaits: its only
/// job is serialising env mutation against this binary's other tests, none of
/// which run on another thread while it is held (clippy cannot see that, so
/// the allowance lives here, at the one async test that needs it).
#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::await_holding_lock)]
async fn serve_runs_and_status_answers() {
    let _lock = lock_env();
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::set(tmp.path());

    // The app's real startup, in-process: init, build, dispatch-as-serve.
    CTX.init(AppKind::Server);
    let (stub, bridge_client) = StubBridge::start().await;
    let runtime = Arc::new(sapphire_framework_server::SyncRuntime::new(
        &CTX,
        bridge_client,
        std::env::current_exe().unwrap(),
        sapphire_ipc::ManagedBy::Service,
    ));
    let server = AppServer::new(&CTX, env!("CARGO_PKG_VERSION")).sync(runtime);
    let endpoint = Endpoint::for_app(CTX.app_name).unwrap();
    let handle = tokio::spawn(async move { server.run().await });
    wait_until_listening(&endpoint).await;

    // The CLI's own verb, in-process: `status` against the running server. The
    // report comes back through the IPC `server.info` method, the same call the
    // framework's `status` command makes and renders.
    let version = env!("CARGO_PKG_VERSION");
    let app_server = AppServer::new(&CTX, version);
    let code = cli::FrameworkCommand::Status
        .dispatch(app_server, version)
        .await
        .unwrap();
    assert_eq!(code, 0, "status against a running server exits 0");

    // Tear the server down the way a test can: `run()` installs no app-side
    // stop channel (the IPC `server.shutdown` method is the framework's), so
    // the spawned task is aborted. Windows' pipe names are released when the
    // listener instance is dropped, which the abort's unwind does before this
    // test's guard is gone — and the next bind is by another process, never a
    // sibling test of this binary.
    handle.abort();
    drop(env);
    drop(stub);
    drop(_lock);
}
