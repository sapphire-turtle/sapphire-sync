//! Two complete sapphire-sync hosts, built from the real pieces.
//!
//! A *host* is a bridge, an app server and a workspace — everything a machine runs, and
//! nothing a machine does not. Two of them on one [`LoopbackNetwork`] are the whole
//! architecture in miniature: client → app server → bridge → peer bridge → peer app
//! server → files, which is what the scenario tests exercise. The harness models the
//! framework's own `sapphire-framework-server/tests/common/mod.rs` — the same structure
//! and the same polling discipline (deadlines and conditions, never a blind sleep) —
//! but with this application's pieces: sapphire-sync's own [`CTX`], the app's marker
//! name, and the wiring `crate::server::build_serve` builds for one host, twice.
//!
//! Everything is in this process rather than spawned. A spawned app server finds its
//! bridge through `SAPPHIRE_RUNTIME_DIR`, and that is one variable in one environment:
//! two hosts cannot hold two values of it. So the harness constructs everything
//! explicitly, one bridge and one app server per host, each against its own runtime
//! directory (`host-<name>/run`) — which is what separates the hosts' IPC endpoints on
//! both carriers: on Unix the endpoint's directory *is* the socket path; on Windows the
//! pipe name is salted with a hash of the directory (framework PR #152), so same-named
//! endpoints in different directories get distinct pipes.
//!
//! Every test file is its own crate, so a fixture one of them does not use is reported
//! as dead code.
#![allow(dead_code)]

/// Emitted once per scenario run, on failure only, ahead of the panic message.
pub fn init_tracing() {
    use tracing_subscriber::fmt::format::FmtSpan;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_span_events(FmtSpan::NONE)
        .try_init();
}

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sapphire_bridge_api::{BridgeClient, GrainId, ManagedBy};
use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::backend::protocol as proto;
use sapphire_sync_core::framework::bridge::{
    Bridge, BridgeDir, LoopbackNetwork, NetConfig, Workgroup,
};
use sapphire_sync_core::framework::ipc::{Client, ClientInfo, Endpoint, connect_or_absent, probe};
use sapphire_sync_core::framework::server::{AppServer, SyncRuntime};
use sapphire_sync_core::framework::workspace::AppContext;
use toml::Table;

/// The node id of the first host: 64 lowercase hex digits, as the ledger wants them.
pub const NODE_A: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";
/// The node id of the second host.
pub const NODE_B: &str = "b1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";

/// The version every side reports, so a client and a server always agree.
pub const VERSION: &str = "0.0.0";

/// This application's context, the same `static` the app's binaries use.
///
/// Its directories are pointed into a scratch tree once per test binary, before any
/// workspace is opened (first writer wins), and the one tree serves every host.
///
/// Sharing one tree is safe, and recording *why* is the point: two verified findings
/// the later tasks rely on.
///
/// - A workspace's cache is `<cache>/<uuid>/`, and the uuid is derived from the
///   *canonicalized root path* (`AppContext::cache_dir_for` → workspace `path_uuid`),
///   so no two workspaces — on one host or on two — land on the same cache directory.
///   The hosts' roots here differ (`host-a/notes` vs `host-b/notes`), so their canonical
///   paths and uuids differ; the same keying applies to the data and config categories.
/// - The sync *identity* is deliberately not path-derived: it is the `sync-id` grain-id
///   inside the workspace's marker directory (`.<app>/sync-id`, `sync/id.rs`), the same
///   bytes on every device. The registry `config.toml` entry is the CLI's local list,
///   not the sync identity — sync needs no registry entry (see [`introduce`]).
pub fn ctx() -> &'static AppContext {
    static DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let dir = DIR.get_or_init(|| tempfile::tempdir().expect("a scratch tree"));
    // Idempotent: only the first call takes effect, and every call passes the same paths.
    CTX.set_cache_dir(dir.path().join("cache"));
    CTX.set_data_dir(dir.path().join("data"));
    CTX.set_config_dir(dir.path().join("config"));
    &CTX
}

/// One machine: its bridge, its app server and its workspace.
pub struct Host {
    /// Kept for the host's whole life; the temp directory is removed on drop.
    tmp: Option<tempfile::TempDir>,
    /// This host's node id on the loopback network.
    node_id: String,
    /// The workspace root. Named `ws` because every RPC that names a workspace names it.
    pub ws: PathBuf,
    /// The bridge directory and the workgroup this host belongs to.
    pub bridge_dir: BridgeDir,
    pub workgroup_id: GrainId,
    /// Where this host's sockets live.
    runtime_dir: PathBuf,
    /// This host's connection to its own app server: the "client" end of the picture.
    pub client: Client,
    /// The host's connection to its own bridge, for fixtures that settle sessions.
    pub bridge_client: Arc<BridgeClient>,
    /// The sync runtime, held so teardown drops it and releases the replica store.
    runtime: Option<Arc<SyncRuntime>>,
    server: Option<tokio::task::JoinHandle<sapphire_sync_core::framework::server::Result<()>>>,
    /// The bridge runs on its own runtime, because a bridge is its own process.
    ///
    /// A bridge is its own *process*: killing it must close the control connections it
    /// already accepted, or an app server would keep talking to a corpse. A runtime is
    /// the only thing that owns those connections — aborting `run` never touches the
    /// `serve_connection` tasks it spawned. So the bridge gets a runtime of its own.
    bridge: Option<tokio::runtime::Runtime>,
}

/// A fresh host with a bridge and an app server of its own, sync wired but not enabled.
pub async fn start_host(net: &LoopbackNetwork, node_id: &str, device_name: &str) -> Host {
    let tmp = tempfile::tempdir().expect("a host tree");
    let ctx = ctx();
    let runtime_dir = tmp.path().join("run");
    std::fs::create_dir_all(&runtime_dir).unwrap();

    let bridge_dir = BridgeDir::at(tmp.path().join("bridge")).unwrap();
    // One workgroup per host, each founded with its own device record — the framework
    // harness's exact shape. The hosts are made able to reach each other not by
    // sharing a workgroup id but by holding the same device ledger: [`introduce`]
    // copies each host's own record into the other's ledger, which is the state a real
    // pairing leaves (the ledger is synced, so every device record travels to every
    // device). Routing is keyed by the workspace's sync-id, not the workgroup id.
    let founded = Workgroup::create(&bridge_dir, "sync-e2e", device_name, node_id).unwrap();

    // The workspace root with its marker directory: the same shape the CLI's
    // `Workspace::find_from` resolution walks to.
    let root = tmp.path().join("notes");
    std::fs::create_dir_all(root.join(format!(".{}", ctx.app_name))).unwrap();
    let root = root.canonicalize().unwrap();

    // The bridge: control, data and the inbound peer loop, on this host's own endpoints.
    //
    // `wake_on_sync` is off deliberately: the exe a route names would be this test
    // binary, and a bridge that started it would spawn a second copy of the test suite.
    let control = Endpoint::in_dir("bridge", runtime_dir.clone());
    let data = Endpoint::in_dir("bridge-data", runtime_dir.clone());
    let bridge = Bridge::new(
        bridge_dir.clone(),
        Arc::new(net.transport(node_id)),
        VERSION,
    )
    .unwrap()
    .net(NetConfig {
        wake_on_sync: false,
        discovery: false,
        relays: Vec::new(),
        use_default_relays: false,
        ..NetConfig::default()
    })
    .control_endpoint(control.clone())
    .data_endpoint(data);
    let bridge_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a runtime for the bridge");
    bridge_runtime.spawn(async move {
        let _ = bridge.run().await;
    });
    wait_until_listening(&control, "the bridge").await;

    // The app server's connection to its own bridge, built explicitly rather than
    // through `BridgeClient::connect`, which would look the bridge up in the process
    // environment — one value, two hosts.
    let (control_client, _) = connect_or_absent(&control, "bridge", client_info("test"))
        .await
        .unwrap()
        .expect("the bridge is listening");
    let bridge_client = Arc::new(BridgeClient::from_client(
        Arc::new(control_client),
        runtime_dir.clone(),
    ));

    // The app server, with sync wired the way `crate::server::build_serve` wires it.
    let endpoint = Endpoint::in_dir(ctx.app_name, runtime_dir.clone());
    let runtime = Arc::new(SyncRuntime::new(
        ctx,
        Arc::clone(&bridge_client),
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("sapphire-sync")),
        ManagedBy::Service,
    ));
    let server = AppServer::new(ctx, VERSION)
        .endpoint(endpoint.clone())
        .sync(Arc::clone(&runtime));
    let server_task = tokio::spawn(async move { server.run().await });
    wait_until_listening(&endpoint, "the app server").await;

    let (client, _) = connect_or_absent(&endpoint, ctx.app_name, client_info("cli"))
        .await
        .unwrap()
        .expect("the app server is listening");

    let workgroup_id = founded.id;

    Host {
        tmp: Some(tmp),
        node_id: node_id.to_owned(),
        ws: root,
        bridge_dir,
        workgroup_id,
        runtime_dir,
        client,
        bridge_client,
        runtime: Some(runtime),
        server: Some(server_task),
        bridge: Some(bridge_runtime),
    }
}

/// This host's node id on the loopback network.
impl Host {
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// The sync runtime, while the app server runs.
    pub fn runtime(&self) -> Option<Arc<SyncRuntime>> {
        self.runtime.clone()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // A test ends here, inside an async context, so the teardown goes to a thread of
        // its own: dropping a multi-threaded runtime is blocking work and tokio refuses
        // to do it on an async worker. Detached rather than joined — this is only
        // releasing the host's own resources, and nothing afterwards depends on it. The
        // drop is what closes the control connections the bridge accepted.
        if let Some(bridge) = self.bridge.take() {
            std::thread::spawn(move || drop(bridge));
        }
    }
}

/// Enable sync on both hosts for one workspace, and make them able to reach each other.
///
/// The two halves of "these hosts share a workspace": the ledger, so each bridge knows
/// the other device, and the `sync-id`, so both servers agree which workspace it is. In
/// the field both arrive by pairing and by the workspace itself being synced; a test has
/// to say so. The sync-id here is the *whole* identity: the registry `config.toml` entry
/// is not consulted by the server at all — the identity is this file, which is why a
/// workspace freshly cloned to another machine syncs without any registry setup.
pub fn introduce(a: &Host, b: &Host) {
    // A workspace's identity is a grain-id in `.<app>/sync-id`, and it travels with the
    // workspace. Minting one per host would make two unrelated workspaces that never
    // merge.
    let shared = GrainId::random();
    for host in [a, b] {
        std::fs::write(sync_id_path(host), format!("{shared}\n")).unwrap();
    }
    copy_device(a, b);
    copy_device(b, a);
}

/// `<root>/.<app>/sync-id`.
fn sync_id_path(host: &Host) -> PathBuf {
    host.ws.join(format!(".{}", ctx().app_name)).join("sync-id")
}

/// Give `to` the record of `from`'s own device — the same id, the same node id.
///
/// The ledger is synced, so a device travels to another host with its id intact; the id
/// is both the record's filename and the `Entry.author` of everything it wrote. Copying
/// the record says exactly that.
///
/// The copy's *name* is prefixed with `~`, the largest printable ASCII character, so it
/// sorts after the receiving host's own record — which matters to a joiner's ledger,
/// where both hosts are joined to one workgroup and a name that sorted first would make
/// the ledger ambiguous about which record is whose. Real resolution never guesses:
/// `Workgroup::this_device` matches on node id, which a record copied wholesale
/// preserves.
fn copy_device(from: &Host, to: &Host) {
    let workgroup = Workgroup::open(&from.bridge_dir).unwrap().unwrap();
    let own = workgroup.this_device(from.node_id()).unwrap();
    let source = from
        .bridge_dir
        .devices_dir(from.workgroup_id)
        .join(own.file_name());
    let text = std::fs::read_to_string(&source).unwrap();
    let mut record: Table = toml::from_str(&text).unwrap();
    let name = record
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("peer")
        .to_owned();
    // `~` is the largest printable ASCII character, so this sorts last whatever the name is.
    record.insert("name".into(), toml::Value::String(format!("~{name}")));

    let destination = to
        .bridge_dir
        .devices_dir(to.workgroup_id)
        .join(own.file_name());
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    std::fs::write(&destination, toml::to_string_pretty(&record).unwrap()).unwrap();
}

/// What a client says it is. The version must match the server's, or the handshake
/// replaces what it finds.
fn client_info(kind: &str) -> ClientInfo {
    ClientInfo {
        kind: kind.to_owned(),
        version: VERSION.to_owned(),
        pid: std::process::id(),
    }
}

/// Wait until something listens on `endpoint`, polling rather than sleeping.
pub async fn wait_until_listening(endpoint: &Endpoint, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !probe(endpoint).await.unwrap_or(false) {
        assert!(Instant::now() < deadline, "{what} never started listening");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Poll `condition` until it holds or the deadline passes.
///
/// The framework harness's discipline, in one helper: every wait names the condition it
/// waits for, and a missed condition is a *deadline*, not a sleep. Never call
/// `std::thread::sleep` in a scenario to "give it time" — a slow machine turns that
/// into a flake, a fast one into dead time.
pub async fn poll_until<F, T>(mut condition: F, what: &str) -> T
where
    F: FnMut() -> Option<T>,
{
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(value) = condition() {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "never converged: {what} (polling gave up after 10s)"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Wait until every host holds an open live session to every peer its bridge reports as
/// connected.
///
/// "Settled" is about the dialer, not the files: the initial exchange of each session is
/// what catches a host up, and a session opens only once both sides have said `Done` and
/// `Settled`. An open session to a peer therefore means "caught up with that peer" — the
/// state the scenario needs before it writes anything, so the writes below are carried
/// live and the polls that follow measure exactly one propagation, not a catch-up.
pub async fn settle(hosts: &[&Host]) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let mut settled = true;
        for host in hosts {
            let Some(runtime) = host.runtime() else {
                settled = false;
                break;
            };
            let peers = host
                .bridge_client
                .peers()
                .await
                .expect("the bridge answers peers");
            let connected: HashSet<GrainId> = peers
                .peers
                .iter()
                .filter(|p| p.connected && p.node_id != host.node_id())
                .map(|p| p.device_id)
                .collect();
            let open: HashSet<GrainId> = runtime
                .live_session_devices(&host.ws)
                .await
                .into_iter()
                .collect();
            if !connected.is_subset(&open) {
                settled = false;
                break;
            }
        }
        if settled {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the hosts never settled into live sessions"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Enable sync on `host`'s workspace, the way a client would.
///
/// `enable` dials immediately ("dial now so enabling converges"), so a workspace enabled
/// on a host whose peer is already running starts its first session without waiting for
/// the dial loop.
pub async fn enable_sync(host: &Host) {
    let _: proto::SyncEnableResult = host
        .client
        .call(
            proto::SYNC_ENABLE,
            proto::WsParams {
                ws: host.ws.clone(),
            },
        )
        .await
        .unwrap();
}

/// Two hosts sharing one workspace, synced, introduced and settled.
///
/// Each host founds its own workgroup; [`introduce`] then gives each host a copy of the
/// other's device record (so each bridge can reach the other's node) and the same
/// `sync-id` (the workspace identity both servers key on). Both then enable sync, which
/// dials immediately. The returned pair has open live sessions; the caller can write
/// from either side.
pub async fn synced_pair(net: &LoopbackNetwork) -> (Host, Host) {
    let a = start_host(net, NODE_A, "host-a").await;
    let b = start_host(net, NODE_B, "host-b").await;
    introduce(&a, &b);
    tokio::time::timeout(Duration::from_secs(20), enable_sync(&a))
        .await
        .expect("enable_sync(a) timed out (20s)");
    tokio::time::timeout(Duration::from_secs(20), enable_sync(&b))
        .await
        .expect("enable_sync(b) timed out (20s)");
    tokio::time::timeout(Duration::from_secs(45), settle(&[&a, &b]))
        .await
        .expect("the hosts never settled (45s timeout)");
    (a, b)
}

/// Write a file into `host`'s workspace, creating parent directories.
///
/// This is an edit made *outside* the server — as if the user had saved from an editor —
/// which is the case the watcher exists for. The scenario then waits for the change to
/// propagate; it never touches the other host's tree directly.
pub fn write(host: &Host, rel: &str, content: &str) {
    let path = host.ws.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

/// Assert the two hosts' workspaces hold exactly the same file tree, file by file.
///
/// The final assertion of the scenario: after the last convergence poll, both roots must
/// agree on every relative path and every file's bytes — the marker directories excluded,
/// since each side's sync state (its replica store, its staging) is deliberately local.
///
/// On Windows the canonical roots spell with `\?\` prefixes; a path printed into the
/// panic message would be noisy but the comparison itself is over relative paths, which
/// spell identically.
pub fn assert_roots_match(a: &Host, b: &Host) {
    let files_of = |root: &Path| -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        for entry in walk(root) {
            let rel = entry
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&entry).unwrap();
            out.push((rel, bytes));
        }
        out.sort_by(|x, y| x.0.cmp(&y.0));
        out
    };
    let left = files_of(&a.ws);
    let right = files_of(&b.ws);
    assert_eq!(
        left.iter().map(|(rel, _)| rel).collect::<Vec<_>>(),
        right.iter().map(|(rel, _)| rel).collect::<Vec<_>>(),
        "the two roots list different files"
    );
    for ((rel, left_bytes), (_, right_bytes)) in left.iter().zip(right.iter()) {
        assert_eq!(left_bytes, right_bytes, "{rel} differs between the hosts");
    }
}

/// Every file under `root`, recursively, skipping the app's marker directory.
fn walk(root: &Path) -> Vec<std::path::PathBuf> {
    let marker = format!(".{}", ctx().app_name);
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                if path
                    .file_name()
                    .map(|n| n != std::ffi::OsStr::new(&marker))
                    .unwrap_or(true)
                {
                    stack.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    out
}
