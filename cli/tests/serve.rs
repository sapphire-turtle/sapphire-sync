//! The serve smoke tests: the binary's `status` against a real server process.
//!
//! The framework guarantees the app: `status` against a running server prints
//! `running: true` (+ pid, exit 0), and against none prints the framework's
//! "no sapphire-sync server is running" (exit 1). Both go through the *real
//! binary* — `env!("CARGO_BIN_EXE_sapphire-sync")` — so the subprocess's
//! stdout and stderr are captured exactly as a user sees them, error format
//! included. The server itself is the in-process `AppServer::run()` of the
//! same pieces `server::build_serve()` wires — `SyncRuntime::new(&CTX,
//! bridge, exe, ManagedBy::Service)` into `AppServer::new(&CTX,
//! VERSION).sync(..)` — except the bridge, which is the `test-util`
//! `StubBridge` (an in-process stand-in: `BridgeClient::connect` needs a
//! running daemon, and starting one is Task 4's e2e harness).
//!
//! The parse side lives in `cli_parse.rs`; the app's own `sync` verb against
//! a live in-process server lives in `cli_sync.rs`. The one binary-vs-crate
//! assertion here is that the compiled-in parse default matches what the
//! binary does.

use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};

use sapphire_framework_server::sync::testing::StubBridge;
use sapphire_ipc::probe;
use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::ipc::Endpoint;
use sapphire_sync_core::framework::server::AppServer;
use sapphire_sync_core::framework::workspace::AppKind;

/// Everything one test needs: the env overrides and their guard.
///
/// The three `SAPPHIRE_SYNC_*` vars route `CTX.init` into the scratch tree,
/// `SAPPHIRE_RUNTIME_DIR` routes the IPC endpoints (and the bridge endpoint)
/// into it.
struct Env {
    previous: [(&'static str, Option<std::ffi::OsString>); 4],
}

impl Env {
    /// Point the context and the runtime directory at `root`, in one lock.
    ///
    /// The previous values are captured before the writes, so the guard
    /// restores the caller's environment (the framework's EnvGuard pattern).
    fn set(root: &std::path::Path) -> Env {
        let names = [
            "SAPPHIRE_SYNC_CACHE_DIR",
            "SAPPHIRE_SYNC_DATA_DIR",
            "SAPPHIRE_SYNC_CONFIG_DIR",
            "SAPPHIRE_RUNTIME_DIR",
        ];
        // SAFETY: ENV_LOCK serialises every read and write of the process
        // environment in this test binary; the writes below happen under the
        // caller's lock, and the guard holds that same lock until Drop has
        // restored the captured values — including while unwinding.
        let previous = unsafe {
            names
                .map(|name| (name, std::env::var_os(name)))
                .map(|(name, old)| {
                    std::env::set_var(name, root.join(leaf_of(name)));
                    (name, old)
                })
        };
        Env { previous }
    }
}

/// The scratch-tree leaf each guarded variable points at.
fn leaf_of(name: &str) -> &'static str {
    match name {
        "SAPPHIRE_SYNC_CACHE_DIR" => "cache",
        "SAPPHIRE_SYNC_DATA_DIR" => "data",
        "SAPPHIRE_SYNC_CONFIG_DIR" => "config",
        _ => "run",
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        // SAFETY: the same ENV_LOCK guard is still held; Drop runs before
        // it is released.
        unsafe {
            for (name, value) in &self.previous {
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
/// The returned guard is kept alive until the end of the test, so the Env
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

/// Run the real binary with the guarded environment inherited, capturing its
/// output. `args` after the program name.
fn run_binary(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sapphire-sync"))
        .args(args)
        .output()
        .expect("the sapphire-sync binary must run")
}

/// `status` with no server: the framework's one line, exit 1.
#[test]
fn status_without_a_server_prints_the_frameworks_line() {
    let _lock = lock_env();
    let tmp = tempfile::tempdir().unwrap();
    let _env = Env::set(tmp.path());

    // A bare CTX.init is not needed by the binary (it does its own), and
    // this test must not init the process's CTX at all: one init per process.
    let output = run_binary(&["status"]);

    assert_eq!(output.status.code(), Some(1), "absent server exits 1");
    // The framework prints the absence line on stdout (a status report, not
    // an error); stderr stays empty.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("no sapphire-sync server is running"),
        "the framework's absence line, got: {stdout}"
    );
}

/// `sync disable` with no server, from a directory a workspace resolves from:
/// the framework's absence line on stdout, exit 1 — the same answer every
/// one-shot verb gives, because dispatch connects and never starts anything.
///
/// The subprocess runs inside a workspace (its cwd is the test's scratch
/// tree): without one the resolution fails before the connect does, and the
/// line the test would pin is the app's resolution error instead of the
/// framework's.
#[test]
fn sync_disable_without_a_server_prints_the_frameworks_line() {
    let _lock = lock_env();
    let tmp = tempfile::tempdir().unwrap();
    let _env = Env::set(tmp.path());
    std::fs::create_dir_all(tmp.path().join("notes").join(format!(".{}", CTX.app_name))).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sapphire-sync"))
        .args(["sync", "disable"])
        .current_dir(tmp.path().join("notes"))
        .output()
        .expect("the sapphire-sync binary must run");

    assert_eq!(output.status.code(), Some(1), "absent server exits 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("no sapphire-sync server is running"),
        "the framework's absence line, got: {stdout}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).is_empty(),
        "absence is a report, not an error: stderr stays empty"
    );
}

/// `status` against a running server: `running: true` + pid, exit 0.
#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::await_holding_lock)]
async fn status_against_a_running_server_prints_running_true() {
    let _lock = lock_env();
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::set(tmp.path());

    // The app's real startup, in-process: init, bridge, build, run.
    CTX.init(AppKind::Server);
    let (stub, bridge_client) = StubBridge::start().await;
    let runtime = Arc::new(sapphire_framework_server::SyncRuntime::new(
        &CTX,
        bridge_client,
        std::env::current_exe().unwrap(),
        sapphire_bridge_api::ManagedBy::Service,
    ));
    let server = AppServer::new(&CTX, env!("CARGO_PKG_VERSION")).sync(runtime);
    let endpoint = Endpoint::for_app(CTX.app_name).unwrap();
    let handle = tokio::spawn(async move { server.run().await });
    wait_until_listening(&endpoint).await;

    // The CLI's own verb, as its own process, so the rendered report is
    // asserted exactly as a user sees it.
    let output = run_binary(&["status"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "status against a running server exits 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("running: true"), "output was: {stdout}");
    assert!(stdout.contains("pid: "), "output was: {stdout}");

    // Tear the server down the way a test can: run() installs no app-side
    // stop channel (the IPC server.shutdown method is the framework's), so
    // the spawned task is aborted. Windows' pipe names are released when the
    // listener instance is dropped, which the abort's unwind does before this
    // test's guard is gone — and the next bind is by another process, never a
    // sibling test of this binary.
    handle.abort();
    drop(env);
    drop(stub);
    drop(_lock);
}
