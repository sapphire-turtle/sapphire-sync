# sapphire-sync App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax with code blocks per step. Do not skip the "run the test and watch it fail" steps: they are what proves the test exercises the new code.

**Goal:** Turn the scaffolded `sapphire-sync` repository into the sync-only reference app:
the smallest possible app server — a workspace with a replica and nothing else — driven by
one binary (`sapphire-sync` with no subcommand runs the server; every other subcommand is a
one-shot CLI call over the socket), and the application-level E2E test bed for the
framework's sync stack.

**Architecture:** The app builds on `sapphire-framework-server` like any other app: it
implements nothing of the replication engine itself. `SyncRuntime` (built by the app with
`SyncRuntime::new(ctx, bridge_client, exe_path, managed_by)`) owns one `Replica` per synced
workspace root, registers them with the bridge, and is mounted by `AppServer::sync(...)` —
which internally mounts `workspace_router_with_sync` + `sync_router`, so the app adds **no
router of its own**. The CLI follows the ledger pattern: a thin clap parse in `cli/src`
dispatching to the core crate; one-shot commands reach the running server through
`IpcBackend::connect` (which runs `ensure_server` internally — the whole spec §2.6
on-demand story, no CLI-side spawn code). The bridge is a *separate process* on the host
(the framework's own `sapphire-bridge` app); the app never owns an iroh endpoint.

**Tech Stack:** Rust 2024 (toolchain 1.98.0, pinned in `rust-toolchain.toml`), the
`sapphire-framework` facade pinned to git branch `feat/p2p-sync-iroh` with
`default-features = false, features = ["server", "sync", "bridge", "redb-store"]` (the
scaffold's scaffold lacks `bridge` — `BridgeClient` lives behind it; adding it is Task 2,
Step 0), clap 4 (derive), tokio 1 (multi-thread, already in the scaffold), anyhow,
tracing + tracing-subscriber (already in the scaffold); dev: tempfile 3.

**Specs (in the framework repository, `docs/superpowers/specs/`):**
- `2026-09-15-sapphire-sync-design.md` — this application. Read §1–§7 **as modified by the
  2026-09-16 superseding note at the top of that file**: no embedded node; the bridge,
  workgroups, pairing, devices and network config belong to `sapphire-bridge`, not here.
- `2026-09-16-process-architecture-design.md` — the process model this app plugs into
  (§2.6 on-demand start, §3 service install, §4 app server, §4.3 the two planes).

**Depends on:** the framework branch `feat/p2p-sync-iroh` at or past `25a64bc`, where all
eleven implementation steps are complete and the final review's findings are fixed on the
branch (`99c62c8` drives the workgroup workspace while the bridge runs — Critical #1;
`0738a04` reports connected peers; `537cd04` verifies the workgroup before marking an owner
online). The branch stays unmerged to `main` on purpose: this repo pins the branch as a git
dependency until the framework releases v0.1.0, then re-pins to the tag (see Global
Constraints).

**Start from:** scaffold commit `ac6a695` on `main` (root workspace with `default-members
= ["cli"]`, `cli/` with a `Command::Run` placeholder binary,
`crates/sapphire-sync-core/`, `rust-toolchain.toml`, dependency pins).

## Deviations from the spec, agreed up front

1. **Two crates, not one flat `src/`.** The spec's §1 layout predates the move out of the
   framework repository. This repo follows the family's established layout (as
   sapphire-ledger): `cli/` (the binary, thin dispatch) + `crates/sapphire-sync-core/` (the
   logic). The command surface of spec §2 is unchanged otherwise.
2. **The node model is superseded** (the note at the top of the spec): no node directory,
   no holder/follower lock, no `net.toml`, no `node status` / `node log`, and no
   `post_install` step (process-architecture spec §3 has no post-install step for this app;
   `AppServer::service_spec()` builds the spec and there is no app-side extra to inject).
   What remains of §2 for this app: bare run / `server run`, `init <path>`,
   `sync enable|map|status`, `server status|stop`, `service install|uninstall|status`.
3. **Bare invocation ≡ `server run`.** `ServerCommand` is flattened under a top-level
   `server` subcommand (a bare `run` at top level cannot parse the unit's `ExecStart`, which
   the framework builds as `<exe> server run`), and bare invocation maps to
   `Run(RunArgs { foreground: .. })` exactly as the bridge's own binary maps no-subcommand
   to `Run`.
4. **This app's server is a daemon.** Unlike journal/ledger (on-demand, idle-exiting
   servers), sapphire-sync's server *is* the product — the dedicated background sync
   service. The plan builds every server with `.idle_exit(None)` (never exit on idle; the
   watcher and live sessions are the point of the process) and
   `.managed_by(ManagedBy::Service)` (a CLI one-shot may still start it —
   `SpawnConfig::default()` keeps `allow_spawn: true` — but it then persists, so a client
   must not shut it down).
5. **`init <path>` takes no `--name`** (the superseded spec's `share_workspace(name)` is
   gone): the workgroup display name is derived from the directory's file name by the
   bridge at registration time (framework `control.rs::register`), so a flag here would
   silently lie. `init` creates the directory, the `.sapphire-sync/` marker, and the
   comment-only `.sapphireignore` template — and nothing else. The `sync-id` file is minted
   by `SyncRuntime::enable` (via `sync::id::sync_id`, which creates a fresh `GrainId` on
   first enable) or written by `SyncRuntime::map` (the workgroup's id); `init` never touches
   it, so a fresh `init` + `enable` is safe *before* a workgroup exists, and `map` later
   refuses to silently rewrite a different id already on disk.
6. **Superseded E2E scenarios.** Old ③ (takeover) is the bridge's own
   wake-on-sync/lock behaviour, tested in the framework; old ⑤ (hosting other apps'
   workspaces) and old ⑥ (standalone `sync` one-shot) are removed by the 2026-09-16 note
   (a host wanting an opaque copy of other apps' files uses rsync or git; sync is driven by
   the running server, live). What this app's E2E keeps: ① pair & propagate, ② conflict
   copy, ③ missing-root guard, ④ filtering & size limit.
7. **E2E harness runs in-process, like the framework's own harnesses**
   (`crates/sapphire-framework-server/tests/common/mod.rs`): each simulated host is a real
   in-process `Bridge` (from `sapphire-framework-bridge`) plus a real `AppServer`, two hosts
   joined over one `LoopbackNetwork`. Spawning real processes per host does not work with
   one process-global runtime dir (the framework's harness comment says why); the
   `SAPPHIRE_BRIDGE_DIR` / static-peer-`net.toml` harness from the old spec belonged to the
   superseded node model.

## Global Constraints

- Code, comments, commit messages and tests in **English**; READMEs in English and
  Japanese, cross-linked at the top (the family `CONTRIBUTING.md` rule).
- CI runs `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --
  -D warnings`, `cargo test --all-features --locked`, and the dependency guard:
  `cargo tree -p sapphire-sync -i sapphire-framework-retrieve` must output **nothing**
  (the scaffold's feature list — `server` (implies `backend`+`workspace`), `sync`,
  `bridge`, `redb-store` — already guarantees it; the check is the guard).
- The framework dependency stays `{ git = "…/sapphire-framework", branch =
  "feat/p2p-sync-iroh", default-features = false, … }` until that branch merges and releases
  v0.1.0, then re-pins to the tag — the only allowed change to the dependency line (and it
  touches `Cargo.lock` in the same commit).
- `AppContext` is `&'static` everywhere: `pub static CTX: AppContext =
  AppContext::new("sapphire-sync");` in the core crate; `CTX.init(kind)` is the *first
  statement of `main`* (first writer wins: `AppKind::Server` in the server path,
  `AppKind::Cli` in one-shot invocations). App name `sapphire-sync` ⇒ marker `.sapphire-sync/`
  and env override `SAPPHIRE_SYNC_DIR`.
- Exit codes: 0 success; 1 runtime error (any `Err` out of `main`, printed as
  `sapphire-sync: {err}` to stderr — the scaffold's `main` shape already does this); 2 usage
  error (clap's default — do not override).
- Every public item in the core crate carries a doc comment; `#![warn(missing_docs)]` on
  `sapphire-sync-core`.

## File structure

```
cli/                              # package `sapphire-sync` (bin, thin dispatch)
  src/main.rs                     # CTX.init first, clap parse, dispatch
  src/server.rs                   # the AppServer: build (bridge+runtime), dispatch ServerCommand
  src/sync.rs                     # sync enable | map | status over IPC
  src/init.rs                     # `init <path>` → core::init (arg parsing only)
  tests/cli_parse.rs              # the whole command surface parses; bare = run
  tests/e2e/common/mod.rs         # the two-host harness
  tests/e2e/propagate.rs          # scenario ① (also ② conflict, ④ filtering)
  tests/e2e/missing_root.rs       # scenario ③
crates/sapphire-sync-core/
  src/lib.rs                      # #![warn(missing_docs)], pub use framework, CTX
  src/init.rs                     # marker + template + idempotency (no marker-dir state beyond these)
  tests/init.rs                   # init unit tests (EnvGuard pattern)
```

---

### Task 1: Core crate — context, init, re-exports

**Files:**
- Modify: `crates/sapphire-sync-core/src/lib.rs`
- Create: `crates/sapphire-sync-core/src/init.rs`
- Test: `crates/sapphire-sync-core/tests/init.rs`

**Interfaces:**
- Consumes: `sapphire_framework::workspace::{AppContext, AppKind, Workspace, WorkspaceArgs}`
  and the scaffold's dependency list (Task 1 does *not* yet need `bridge`; leave it for
  Task 2's Step 0 so this task's diff stays reviewable).
- Produces:
  - `sapphire_sync_core::CTX: AppContext` — `pub static CTX: AppContext = AppContext::new("sapphire-sync");`
  - `sapphire_sync_core::init::init(path: &Path) -> anyhow::Result<()>` — create `path` if
    missing (`create_dir_all`), create `path/.sapphire-sync/` (idempotent), write the
    comment-only `.sapphireignore` template **only if none exists**; never touches
    `sync-id`, never calls `Workspace::from_root*` (the workspace *store* opens when the
    server enables sync, not here).
  - `sapphire_sync_core::init::TEMPLATE: &str` — the template body, a comment-only file:
    what the file does, that it uses gitignore syntax, that it is synced to every device,
    and the two commented-out examples `# *.tmp` and `# node_modules/`.
  - `sapphire_sync_core` re-exports: `pub use sapphire_framework as framework;` (the CLI
    depends on the core crate only; the facade rides along).

- [ ] **Step 1: Write the failing tests.** `tests/init.rs`, env-guarded exactly like the
      framework's `sync/methods.rs` tests (a `static CTX` for this app, a `Mutex` serialising
      env, an `EnvGuard` restoring `SAPPHIRE_SYNC_{CACHE,DATA,CONFIG}_DIR` on drop —
      copy that pattern, `unsafe` set/remove under the one lock). Tests, each on a fresh
      `tempfile::TempDir`:
      1. `init` creates the directory when it is missing, creates `.sapphire-sync/`, and
         writes a `.sapphireignore` containing `# ` lines and the two example patterns.
      2. `init` twice is a no-op: same marker exists, no error, the template is not rewritten.
      3. a pre-existing `.sapphireignore` is left byte-identical (write `*.bak\n` first).
      4. a second `init` at a path under an existing workspace errors (the marker would be
         nested): document-and-assert the actual `Workspace`/marker behaviour you implement
         — `init` must at minimum not overwrite the outer workspace's marker.
- [ ] **Step 2: Run, watch them fail:** `cargo test -p sapphire-sync-core` (unresolved
      imports `init`/`CTX`).
- [ ] **Step 3: Implement** `CTX`, `init.rs` per the interface above (a dozen lines:
      `create_dir_all` twice, `if !template_path.exists() { fs::write(...) }`, anyhow
      contexts naming every path). Re-export the facade in `lib.rs`.
- [ ] **Step 4: Run, watch them pass; then** `cargo clippy --all-targets --all-features -- -D warnings`
      and `cargo fmt --all -- --check` clean.
- [ ] **Step 5: Commit** — `git add crates/sapphire-sync-core && git commit -m "feat: workspace context and init (marker, template ignore file, idempotency)"`.

---

### Task 2: The server — bare run, `ServerCommand`, mounted sync

**Files:**
- Modify: `cli/Cargo.toml` (add `bridge` to the framework features; add dev-dependency
  `sapphire-framework-server` on the *same git source* with `features = ["test-util"]` —
  the facade does not expose `test-util`, so the StubBridge comes from the crate directly)
- Modify: `cli/src/main.rs`
- Create: `cli/src/server.rs`
- Test: `cli/tests/cli_parse.rs`

**Interfaces:**
- Consumes: `CTX` from Task 1; the framework facade: `server::{AppServer, ServerCommand, RunArgs, spawn_config_for}`,
  `server::sync::SyncRuntime` (`new(ctx, bridge, exe_path, managed_by)` — registers on
  enable; `enable` dials immediately), `bridge::BridgeClient` (`connect(kind, version,
  &SpawnConfig) -> Result<BridgeClient>`, connecting to `Endpoint::for_bridge()`),
  `ipc::{Endpoint, ManagedBy, SpawnConfig}`, `workspace::args::WorkspaceArgs`.
- Produces:
  - `cli/src/main.rs` `Cli`:
    ```rust
    #[derive(Parser)]
    #[command(name = "sapphire-sync", version, about = "…")]
    struct Cli {
        #[command(flatten)]
        workspace: WorkspaceArgs,           // global --workspace-dir (alias --sync-dir not needed)
        #[command(subcommand)]
        command: Option<Command>,
    }
    #[derive(Subcommand)]
    enum Command {
        /// Manage this app's server. Bare invocation is equivalent to `server run`.
        #[command(subcommand)]
        Server(ServerCommand),              // framework type: run | status | stop | service …
        Init { /* Task 1 */ },
        Sync { #[command(subcommand)] command: SyncCommand },   // Task 3
    }
    ```
    `main`: `CTX.init(match command { Some(Server(Run(_)))|None => Server, _ => Cli })`,
    then `Command::Server(c) => server::dispatch(c).await`.
  - `cli/src/server.rs`:
    ```rust
    /// Build this app's server: bridge connection + SyncRuntime, `idle_exit(None)`,
    /// `managed_by(Service)` (Deviation 4), default `SpawnConfig` for on-demand starts.
    pub async fn build() -> anyhow::Result<AppServer>;
    pub async fn dispatch(command: ServerCommand) -> anyhow::Result<i32> {
        let server = build().await?;
        Ok(ServerCommand::dispatch(command, server, "sapphire-sync", env!("CARGO_PKG_VERSION")).await?)
    }
    ```
    `build()` = `BridgeClient::connect("sapphire-sync", VERSION, &SpawnConfig::default())`
    → `SyncRuntime::new(&CTX, Arc::new(bridge), std::env::current_exe()?, ManagedBy::Service)`
    → `AppServer::new(&CTX, VERSION).managed_by(ManagedBy::Service).idle_exit(None).sync(arc)`.
    The bare/`run` path *is* `dispatch(Run(args))`; the service unit's `ExecStart = <exe> server run`
    lands on the same code path (Deviation 3).
- A `ServerCommand::Service(_)` dispatch builds the whole server (cheap) because the
  framework's `ServerCommand::dispatch` reads the spec off the `AppServer` — note this in
  the doc comment so the next reader does not "optimise" it away.

- [ ] **Step 0:** add `bridge` to the framework features in `cli/Cargo.toml` and the
      `sapphire-framework-server` dev-dependency (same git source, branch pin,
      `default-features = false`, `features = ["test-util"]`); `cargo check` passes.
- [ ] **Step 1: Write the failing parse test** `cli/tests/cli_parse.rs`, modelled on the
      framework's `command::tests::the_subcommands_parse` (a `#[derive(Parser)] struct Cli`
      re-declaring the real tree is not possible across crates — export `Cli` and
      `Command` from the bin crate via a small `cli` module compiled as both bin and test,
      or simply `assert_cmd`-free: parse via `Cli::try_parse_from` on a `Cli` re-exported
      from `sapphire_sync::cli`): bare `["sapphire-sync"]` → `None`; `["sapphire-sync",
      "server", "run"]`, `… "server", "run", "--foreground"`, `… "server", "status"`,
      `… "server", "stop"`, `… "server", "service", "install"`, `… "server", "service",
      "uninstall"`, `… "server", "service", "status"` all parse; `--workspace-dir /tmp/x`
      is accepted globally.
- [ ] **Step 2: Watch it fail** (`cargo test -p sapphire-sync --test cli_parse`).
- [ ] **Step 3: Implement** `main.rs` (dispatch shape above; the `Init`/`Sync` arms dispatch
      to functions delivered by Tasks 1/3 — land this task *after* 1 or stub their arms with
      `anyhow::bail!("not implemented")` **only if** Task 3 has not landed; order the work
      1 → 3 → 2 if needed) and `server.rs` (the two functions above). One smoke integration
      test `cli/tests/server_run.rs`: point `SAPPHIRE_SYNC_CACHE_DIR/_DATA_DIR/_CONFIG_DIR`
      and the IPC runtime dir at a temp dir (`Endpoint::for_app` resolves from the runtime
      dir — set `SAPPHIRE_RUNTIME_DIR` per the env-guard pattern), spawn `build()` in a
      task, `wait_until_listening` (poll `sapphire_ipc::probe`, pattern from the
      framework's `sync_wiring.rs`), assert `ServerCommand::Status` reports it, that
      `Stop` stops it, then teardown.
- [ ] **Step 4: All green; fmt + clippy clean; commit** — `"feat: embedded app server (bare run, ServerCommand, mounted SyncRuntime)"`.

---

### Task 3: One-shot CLI — `sync enable | map | status | disable`

**Files:**
- Create: `cli/src/sync.rs`
- Modify: `cli/src/main.rs` (the `Sync` arm)
- Test: `cli/tests/cli_sync.rs`

**Interfaces:**
- Consumes: `IpcBackend::connect(&Endpoint::for_app("sapphire-sync")?, "sapphire-sync",
  "cli", VERSION, &SpawnConfig::default(), root)` — connect-or-start; `backend.client()
  .call::<_, T>(method, params)` for the framework's `backend::protocol::{SYNC_ENABLE,
  SYNC_DISABLE, SYNC_MAP, SYNC_STATUS, WsParams, SyncMapParams, SyncEnableResult,
  SyncStatusResult}`; `Workspace::resolve(&CTX, cli.workspace.workspace_dir.as_deref())`
  to find the root for `enable`/`disable`/`status` (upward `.sapphire-sync/` search, exactly
  as ledger resolves `--ledger-dir`).
- Produces:
  ```rust
  pub enum SyncCommand {
      /// Start syncing the workspace this command resolves to.
      Enable,
      /// Stop syncing it. Files and sync-id stay; re-enabling rejoins the same workspace.
      Disable,
      /// Place an existing workgroup workspace at <dir> and sync it.
      Map {
          /// The workspace, by name or id, as the workgroup lists it.
          workspace: String,
          /// The directory it lives in on this host (must already be a workspace).
          dir: PathBuf,
      },
      /// Print what this host syncs: enabled, id, peers, paused, last error, bridge.
      Status,
  }
  pub async fn dispatch(command: SyncCommand, root: Option<PathBuf>) -> anyhow::Result<i32>;
  ```
  `map` passes `SyncMapParams { workspace, dir }` verbatim (the server canonicalizes and
  refuses non-roots/wrong-app ids itself — surface the server's error text, do not
  re-validate). `status` prints one human-readable block (key: value lines — this is a CLI,
  no `--json`); `bridge_available: false` prints as a plain line, never an error exit.
  Exit: `Ok(0)` on success; any `Err` propagates to `main` → exit 1.

- [ ] **Step 1: Write the failing tests.** `cli_parse.rs` gains: `sync enable`,
      `sync disable`, `sync status`, `sync map notes /srv/sync/notes` all parse (including
      with a global `--workspace-dir`). `cli_sync.rs`: start an in-process server per
      Task 2's smoke fixture + `sapphire_framework_server::sync::testing::StubBridge`
      (`StubBridge::start()` gives `(StubBridge, Arc<BridgeClient>)` over a
      `Connection::pair`), build the server around the runtime, then run the CLI-level
      dispatch against it: enable a temp workspace → `status` reports `enabled: true` and a
      workspace id; the stub recorded a registration; `disable` → `enabled: false` (the
      stub's unregistrations grow); a *second* enable of the same root is a no-op re-register
      (the framework re-registers the whole set — assert no error and one id).
- [ ] **Step 2: Watch them fail.**
- [ ] **Step 3: Implement** `sync.rs` + the `main.rs` arm; passing.
- [ ] **Step 4: fmt/clippy/tests; commit** — `"feat: sync enable/map/status/disable over the IPC socket"`.

---

### Task 4: E2E harness + scenario ① (pair and propagate)

**Files:**
- Create: `cli/tests/e2e/common/mod.rs`
- Test: `cli/tests/e2e/propagate.rs`

**What the harness is (Deviation 7):** model it on the framework's
`crates/sapphire-framework-server/tests/common/mod.rs` (read it first; it already solves
"two hosts, one process" — one `LoopbackNetwork` shared by two `Arc<Bridge>`s each with its
own `BridgeDir::at(host_dir)` + `Endpoint::in_dir` control/data endpoints, one shared
`static CTX: AppContext = AppContext::new("sapphire-sync")` whose per-uuid cache dirs keep
the hosts' caches apart). Per host: one `Bridge::new(dir, transport, "0.0.0")` run in a
spawned task, one `AppServer` (built exactly like Task 2's `build()`, but per host:
`SyncRuntime::new(&CTX, bridge_client, "/bin/true".into(), ManagedBy::Spawned)`), one
workspace root (`tmp/host{a,b}/notes/` with `.sapphire-sync/sync-id` written identically on
both hosts before enable — the harness writes the id file directly rather than running the
CLI, and pins the workspace cache key with `Workspace::from_root_with_uuid(&CTX, root, uuid)`
so both hosts share one uuid/cache key — **verify** whether the in-process two-host
fixture needs this before choosing; `sync_id`'s shared file guarantees the id, the uuid
only names the cache dir — the framework harness comment confirms one scratch
tree is safe precisely because each workspace cache is keyed by the root path
(`<cache>/<uuid>/`); **record the verified finding in the harness doc comment**, since
Task 5 and later readers rely on it). One workgroup: create it once (`Workgroup::create` on host A's
dir with a fixed `NODE_A` node id; host B joins via the shared directory the same way the
framework's `adopt_workgroup`/`refresh_workgroup` fixtures do), then
`SyncRuntime::enable` on both hosts; `enable`'s dial makes the first session immediate.
Helpers: `write(host, rel, content)`, `poll_until(|| condition, "what")` (10 s timeout,
25 ms poll — copy from the framework harness; **no sleeps without a condition**).

**Scenario ①** (`propagate.rs`): enable on both hosts → write a file on A → it appears
under B's root (poll); modify on B → A converges; delete on A → gone from B. Then the
same file tree on both sides is the assertion (file-by-file compare of the two roots).

- [ ] **Step 1:** write the harness + `propagate.rs` (the scenario above, three `poll_until`s).
- [ ] **Step 2:** run, watch it fail to compile, then fail the first poll.
- [ ] **Step 3:** implement the harness until the scenario passes:
      `cargo test -p sapphire-sync --test e2e` — wait, integration-test-per-file means the
      file is its own test crate (`tests/e2e/propagate.rs` + `mod common` via
      `#[path = "../e2e/common/mod.rs"] mod common;` if `common` must not become its own
      test crate — mirror however the framework's `tests/converge.rs` includes `common/`).
- [ ] **Step 4:** commit — `"test: two-host e2e harness and propagate scenario"`.

---

### Task 5: E2E scenarios — conflict, missing root, filtering & limits

**Files:**
- Test: `cli/tests/e2e/conflict.rs`, `cli/tests/e2e/missing_root.rs`, `cli/tests/e2e/filtering.rs`
  (all via `mod common` from Task 4)

**Scenarios:**
- **Conflict copy:** both hosts enabled, file `notes/a.md` synced with the same content,
  then write *different* content on A and on B (order irrelevant — both are committed),
  poll until both roots converge to one winner plus exactly one conflict copy whose name
  matches `a.conflict-<7-char grain-id>-<n>.md` on both hosts (the framework
  `merge::conflict_path` naming: `*.conflict-<grain-id>-<n>.*`).
- **Missing-root guard:** rename a synced root away on A → A's `sync.status` reports
  `paused: Some("RootMissing")` (the `PauseReason` Debug form — assert on the *string the
  CLI prints*, via Task 3's dispatch against the in-process server, or on the
  `SyncStatus` struct if the harness calls the runtime directly); the peer's files are
  untouched; rename back; `status.paused` clears and a new file written on B reaches A.
- **Filtering & limits:** a `.sapphireignore` (created *before* enable on A; it syncs with
  everything else — assert it exists on B too) excluding `*.tmp` → a `x.tmp` written on A
  never reaches B; a file larger than `sapphire_framework::sync::DEFAULT_MAX_FILE_SIZE` (64 MiB,
  re-exported at the sync crate root; the `replica` module itself is private) is
  skipped (absent on B) while its smaller sibling syncs; `status` shows no pause (skips are
  silent in `sync.status` — the skipped-file assertions above are the real checks).

- [ ] **Steps per file:** write → watch fail → implement the harness helpers the scenario
      still needs (e.g. rename-root helper, oversized payload builder:
      `DEFAULT_MAX_FILE_SIZE + 1` bytes) → passing → fmt/clippy.
- [ ] **Commit** — `"test: conflict, missing-root and filtering e2e scenarios"`.

---

### Task 6: READMEs, CI, CONTRIBUTING

**Files:** `README.md`, `README.ja.md`, `.github/workflows/ci.yml`, `CONTRIBUTING.md`.

- [ ] **README (en + ja, cross-linked at the top).** Sections per spec §4 *as revised*:
      quick start (`sapphire-sync init ~/Documents/notes` → `sapphire-sync sync map notes
      ~/Documents/notes` → it appears in the bridge's `workspace list` on every device and
      two-way sync begins); how the pieces fit (this app = the app server + CLI, the
      `sapphire-bridge` app = the always-on peer and switchboard; one `sapphire-sync server`
      per host); `server service install` (user units, Linux system units run as the
      invoking user); `.sapphireignore`; how conflicts appear (the `.conflict-…` copies and
      why both versions survive); `server status` / `sync status`. Do not assert the
      bridge app's own command names in prose if they differ — link to the bridge README
      instead of restating its CLI.
- [ ] **CI:** the four CI commands from Global Constraints as one workflow
      (ubuntu + windows matrix; `cargo tree -p sapphire-sync -i sapphire-framework-retrieve`
      must print nothing — assert empty).
- [ ] **`CONTRIBUTING.md`:** this repository's rule (two-crate layout `cli/` +
      `crates/sapphire-sync-core/`, the thin-CLI rule, English code/comments/commits,
      tests live in `cli/tests/`); link the framework repo's `CONTRIBUTING.md` for the
      family-wide rules instead of copying them.
- [ ] **Commit** — `"docs: READMEs (en/ja), CI, contributing"`.

---

## Out of scope (unchanged from spec, incl. superseded parts)

Send-only/receive-only folders, file versioning, a web UI, a desktop/tray app (a future
separate crate), binary release artifacts. Workgroup/device/network commands live in
`sapphire-bridge`, not here (superseding note, bullet 3).
