//! The app's `sync` verb against a real in-process server: status rows and the
//! disable round trip.
//!
//! One in-process host, the way `serve.rs` builds it but with the bridge
//! replaced by the server crate's `test-util` `StubBridge` (a stand-in over a
//! `Connection::pair`: `BridgeClient::connect` would need a running daemon, and
//! starting one is Task 4's e2e harness). The test drives the very pieces
//! `server::build_serve()` wires — `SyncRuntime` mounted with
//! `AppServer::sync`, the app's rows mounted with `AppServer::status_rows` —
//! and the app's own `sync::dispatch`, so the assertions cover the wiring, not
//! a copy of it.
//!
//! The dispatch and row types are reached through `#[path]` includes of the
//! same modules `main.rs` pulls in: a `[[bin]]`-only package has no library
//! target, so the integration test compiles them into its own crate (the
//! `cli_parse.rs` pattern).
//!
//! The server's process cwd is the test's scratch tree for the whole body: the
//! rows name the workspace the *server's* current directory resolves to, and
//! this test's server is in-process.

// The modules are compiled into this test crate directly (no lib target to
// link against). `sync.rs` reaches `SyncCommand` through `crate::cli`, which
// resolves to the include below in exactly the same way it resolves inside the
// binary.
#[path = "../src/cli.rs"]
mod cli;
#[path = "../src/sync.rs"]
mod sync;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use sapphire_bridge_api::ManagedBy;
use sapphire_framework_server::sync::testing::StubBridge;
use sapphire_framework_server::{AppServer, StatusRow, SyncRuntime};
use sapphire_ipc::probe;
use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::backend::protocol as proto;
use sapphire_sync_core::framework::ipc::Endpoint;
use sapphire_sync_core::framework::workspace::AppKind;

/// Everything one test needs: the env overrides and their guard.
///
/// The three `SAPPHIRE_SYNC_*` vars route `CTX.init` into the scratch tree,
/// `SAPPHIRE_RUNTIME_DIR` routes the IPC endpoints into it. The previous
/// values are captured *before* the writes (the framework's `EnvGuard`
/// pattern), so the guard restores the caller's environment — including while
/// unwinding from a panic.
struct Env {
    previous: [(&'static str, Option<std::ffi::OsString>); 4],
}

impl Env {
    /// Point the context and the runtime directory at `root`, in one lock.
    fn set(root: &std::path::Path) -> Env {
        let names = [
            "SAPPHIRE_SYNC_CACHE_DIR",
            "SAPPHIRE_SYNC_DATA_DIR",
            "SAPPHIRE_SYNC_CONFIG_DIR",
            "SAPPHIRE_RUNTIME_DIR",
        ];
        // SAFETY: `ENV_LOCK` serialises every read and write of the process
        // environment in this test binary; the writes below happen under the
        // caller's lock, and the guard holds that same lock until `Drop` has
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
        // SAFETY: the same `ENV_LOCK` guard is still held; `Drop` runs before
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
/// A poisoned lock only means some other test panicked while holding it; the
/// environment is not invariant-critical here.
fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Wait until something is listening on `endpoint`.
///
/// `tokio::spawn(server.run())` only schedules the server; without this wait a
/// fast first probe would land before `run` has bound the socket.
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

/// A workspace root with the app's marker directory, canonicalised.
///
/// Nothing else: sync needs the marker and the sync id `enable` mints — the
/// registry is the CLI's local list, not sync's (Task 4 records the finding).
fn workspace_root(tmp: &std::path::Path, name: &str) -> PathBuf {
    let root = tmp.join(name);
    std::fs::create_dir_all(root.join(format!(".{}", CTX.app_name))).unwrap();
    root.canonicalize().unwrap()
}

/// The whole disable round trip, in one scenario: one in-process host, and the
/// `CTX` static's directories are set once per process (first writer wins), so
/// the phases share one fixture instead of each initialising their own.
#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::await_holding_lock)]
async fn disable_unregisters_flips_the_row_and_reenables_with_the_same_id() {
    let _lock = lock_env();
    let tmp = tempfile::tempdir().unwrap();
    let _env = Env::set(tmp.path());
    CTX.init(AppKind::Server);

    let root = workspace_root(tmp.path(), "notes");
    // The server's cwd is the workspace: the row the *server* reports for the
    // current directory is this workspace's (and this test's server is
    // in-process). `SetCwd` restores it on drop, before the env guard goes.
    let _cwd = SetCwd::to(&root);

    // The host: the real runtime over the stub bridge, mounted with the real
    // status rows from the app's `sync` module — the `build_serve` wiring.
    let (stub, bridge_client) = StubBridge::start().await;
    let runtime = Arc::new(SyncRuntime::new(
        &CTX,
        bridge_client,
        std::env::current_exe().unwrap(),
        ManagedBy::Service,
    ));
    let rows = sync::status_rows(Arc::clone(&runtime));
    let rows = Arc::clone(&rows) as Arc<dyn Fn() -> Vec<StatusRow> + Send + Sync>;
    let server = AppServer::new(&CTX, env!("CARGO_PKG_VERSION"))
        .sync(Arc::clone(&runtime))
        .status_rows(Arc::clone(&rows));
    let endpoint = Endpoint::for_app(CTX.app_name).unwrap();
    let handle = tokio::spawn(async move { server.run().await });
    wait_until_listening(&endpoint).await;

    // Nothing is synced, and the resolved current-directory workspace is not
    // either: it is the one `not synced` row.
    assert_eq!(
        rows(),
        vec![StatusRow {
            name: "notes".into(),
            value: "not synced".into(),
        }],
        "the resolved workspace shows as not synced before anything is enabled"
    );

    // Enable: the same row flips to the synced text. The stub's ledger
    // answers `peers` with an empty list — the stub plus this host is the
    // whole workgroup, and the framework's saturating_sub(1) convention turns
    // that into 0 other devices. Assert exactly what the row renders.
    let id = runtime.enable(&root).await.unwrap();
    assert_eq!(
        rows(),
        vec![StatusRow {
            name: "notes".into(),
            value: format!("synced as {id}, 0 peers"),
        }],
        "an enabled workspace is one row: name, id, peer count"
    );

    // Disable through the app's own dispatch, over IPC to this server: the
    // workspace's registration leaves the bridge, and the row flips back.
    let code = sync::dispatch(
        cli::SyncCommand {
            command: cli::SyncSubcommand::Disable,
        },
        Some(root.clone()),
    )
    .await
    .unwrap();
    assert_eq!(code, 0, "a successful disable exits 0");
    assert_eq!(
        stub.seen.lock().expect("stub").unregistrations,
        vec![id],
        "the disabled workspace is unregistered with the bridge"
    );
    assert_eq!(
        rows(),
        vec![StatusRow {
            name: "notes".into(),
            value: "not synced".into(),
        }],
        "the row flips once the runtime no longer syncs the root"
    );

    // Idempotent: disabling an unsynced workspace is still a success, and it
    // does not unregister anything again.
    let code = sync::dispatch(
        cli::SyncCommand {
            command: cli::SyncSubcommand::Disable,
        },
        Some(root.clone()),
    )
    .await
    .unwrap();
    assert_eq!(code, 0, "an idempotent disable exits 0 too");
    assert_eq!(
        stub.seen.lock().expect("stub").unregistrations,
        vec![id],
        "nothing left to unregister"
    );

    // A directory no workspace can be resolved from is the caller's mistake:
    // the error propagates to `main` and exits 1.
    let elsewhere = tempfile::tempdir().unwrap();
    let result = sync::dispatch(
        cli::SyncCommand {
            command: cli::SyncSubcommand::Disable,
        },
        Some(elsewhere.path().to_owned()),
    )
    .await;
    assert!(result.is_err(), "no workspace to resolve, so an error");

    // Re-enable the way `workspace init --sync` does — the framework's
    // `sync.enable` method over IPC. The sync id file stayed on disk, so the
    // workspace rejoins under the same identity, and the row says so.
    let info = sapphire_ipc::ClientInfo {
        kind: "test".into(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        api: proto::API_VERSION,
        pid: std::process::id(),
    };
    let (client, _) = sapphire_ipc::connect_or_absent(&endpoint, CTX.app_name, info)
        .await
        .unwrap()
        .unwrap();
    let enabled: proto::SyncEnableResult = client
        .call(proto::SYNC_ENABLE, proto::WsParams { ws: root.clone() })
        .await
        .unwrap();
    assert_eq!(
        enabled.workspace_id, id,
        "re-enabling rejoins the same workspace, not a second one"
    );
    assert_eq!(
        rows(),
        vec![StatusRow {
            name: "notes".into(),
            value: format!("synced as {id}, 0 peers"),
        }],
        "the row is back, same id"
    );

    // The paused branch, exactly as the code renders it: the row's reason is
    // the framework's `PauseReason` debug form. The replica pauses when its
    // root's marker directory disappears while it has materialized files —
    // here the workspace's file, which the enable scan recorded.
    std::fs::write(root.join("paper.txt"), "words\n").unwrap();
    runtime.scan(&root).await.unwrap();
    std::fs::remove_dir_all(root.join(format!(".{}", CTX.app_name))).unwrap();
    runtime.scan(&root).await.unwrap();
    assert_eq!(
        rows(),
        vec![StatusRow {
            name: "notes".into(),
            value: format!("synced as {id}, 0 peers, paused: MarkerMissing"),
        }],
        "a paused workspace appends the framework's reason"
    );

    // Tear the server down the way a test can: `run()` installs no app-side
    // stop channel, so the spawned task is aborted. Windows' pipe names are
    // released when the listener instance is dropped, which the abort's unwind
    // does before this test's guards are gone.
    handle.abort();
    drop(stub);
    drop(_cwd);
    drop(_env);
    drop(_lock);
}

/// The test process's cwd for the guard's life, restored on drop.
///
/// `std::env::set_current_dir` is process-global, like the environment: the
/// same `ENV_LOCK` that serialises the env vars serialises this, and the
/// caller holds that lock for the whole test.
struct SetCwd {
    previous: PathBuf,
}

impl SetCwd {
    /// Change into `dir`, remembering where the process was.
    fn to(dir: &std::path::Path) -> SetCwd {
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir).unwrap();
        SetCwd { previous }
    }
}

impl Drop for SetCwd {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.previous).unwrap();
    }
}
