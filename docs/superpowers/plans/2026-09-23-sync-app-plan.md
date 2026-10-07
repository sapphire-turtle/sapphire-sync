# sapphire-sync App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or subagent-driven-executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax with code blocks per step. Do not skip the "run the test and watch it fail" steps: they are what proves the test exercises the new code.
>
> **Revised 2026-09-26** for the framework's merged command system (issues #146/#150,
> PR #150 = branch tip `5780e0c`): `FrameworkCommand` now provides
> `serve / status / service / workspace / workgroup / device` flat, start-on-demand is
> gone, and `workspace init|list|map` is framework-provided. The app CLI shrinks to
*almost nothing*; this plan was rewritten where it disagreed with the merged framework.

**Goal:** Turn the scaffolded `sapphire-sync` repository into the sync-only reference app:
the smallest possible app server — a workspace with a replica and nothing else — driven by
one binary (`sapphire-sync` with no subcommand runs the server; every other subcommand is a
one-shot CLI call over the socket), and the application-level E2E test bed for the
framework's sync stack.

**Architecture:** The app builds on `sapphire-framework-server` like any other app: it
implements nothing of the replication engine itself. `SyncRuntime` (built by the app with
`SyncRuntime::new(&CTX, bridge_client, current_exe, ManagedBy::Service)`) owns one `Replica`
per synced workspace root, registers them with the bridge, and is mounted by
`AppServer::sync(...)` — which internally mounts `workspace_router_with_sync` +
`sync_router`, so the app adds **no router of its own** (the only router extension is the
`status_rows` closure, below). The CLI is a thin clap parse that flattens the framework's
`FrameworkCommand` beside the app's own (one-variant) subcommand enum and hands the whole
thing to `FrameworkCommand::dispatch(server, version)`; one-shot commands reach the running
server over IPC via `connect_or_absent` — **nothing is ever started on demand**: no server
→ one line + exit 1. The bridge is a *separate process* on the host (the framework's own
`sapphire-bridge` app, whose CLI has been re-homed onto the same vocabulary by PR #150);
the app never owns an iroh endpoint.

**Tech Stack:** Rust 2024 (toolchain 1.98.0, pinned in `rust-toolchain.toml`), the
`sapphire-framework` facade pinned to git branch `feat/p2p-sync-iroh` with
`default-features = false, features = ["server", "sync", "bridge", "redb-store"]` (the
scaffold lacks `bridge` — `BridgeClient` lives behind it; adding it is Task 2, Step 0),
clap 4 (derive), tokio 1 (multi-thread, already in the scaffold), anyhow,
tracing + tracing-subscriber (already in the scaffold); dev: tempfile 3.

**Specs (in the framework repository, `docs/superpowers/specs/`):**
- `2026-09-15-sapphire-sync-design.md` — this application. Read §1–§7 **as modified by the
  2026-09-16 superseding note at the top of that file**: no embedded node; the bridge,
  workgroups, pairing, devices and network config belong to `sapphire-bridge`, not here.
- `2026-09-16-process-architecture-design.md` — the process model; **its decision 5
  (start-on-demand) is itself superseded** by the command-system spec below: servers are
  always-on daemons, and one-shot commands never start one.
- `2026-09-24-app-command-system-design.md` — the command system this app consumes
  (Phase 1 = PR #146, landed; Phase 2 = the bridge CLI re-homing, landed). Its decisions
  1–8 are implemented on the branch this repo pins; the app-side migration is *this repo*.

**Depends on:** the framework branch `feat/p2p-sync-iroh` at or past `5780e0c` (PR #150,
which merges PR #146: the flat `FrameworkCommand`, the typed `StatusReport`,
`workspace init/list/map` over the app server's IPC, SIGTERM/SIGINT handling in
`AppServer::run`, removal of start-on-demand, and the bridge CLI re-homing — on top of the
P2P sync steps: `99c62c8` Critical #1, `0738a04` connected peers, `537cd04` workgroup
verification). The branch stays unmerged to `main` on purpose: this repo pins the branch as
a git dependency until the framework releases v0.1.0, then re-pins to the tag (see Global
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
   `post_install` step (the installed unit is `AppServer::service_spec()`'s, whose
   `args: ["serve"]` is now hard-coded by the framework itself — the former "hardcoded
   `["server", "run"]"` deviation is closed by #142). What remains of §2 for this app:
   `serve` (bare invocation means the same — `FrameworkCommand::Serve` is `#[default]`),
   a top-level `status` (framework-rendered, app rows injected), `service install|uninstall`
   (the framework's `ServiceCommand`; `stop` is dropped per command-system decision 4 —
   stopping is the service manager's job; the framework now also provides `service status`
   and the app takes it for free), and the workspace verbs (Deviation 5).
3. **The command surface is the framework's, not the app's.** `FrameworkCommand`
   (`Serve | Status | Service | Workspace | Workgroup | Device`) is flattened into the app's
   clap parser beside the app's own single subcommand (Deviation 6). The app defines **no**
   `serve`/`status`/`service`/`workspace`/`workgroup`/`device` types of its own; it builds an
   `AppServer` and hands every framework command to
   `FrameworkCommand::dispatch(self, server, version)` (the `AppServer` rides along for
   `serve`/`status`/`service` — the bridge-less one-shot build is cheap and that is the shape
   the framework's dispatch takes; do not "optimise" it away). `ServiceStatus` comes with
   the framework's `ServiceCommand` and is *included* here. Start-on-demand is gone
   (`SpawnConfig` is gone with it): every one-shot verb either finds the running server or
   prints "no sapphire-sync server is running" and exits 1.
4. **This app's server is a daemon.** Unlike journal/ledger (historically on-demand,
   idle-exiting servers), sapphire-sync's server *is* the product — the dedicated background
   sync service. The framework now hard-wires this: `AppServer::run` no longer idles out and
   always reports `ManagedBy::Service` (the `managed_by` builder method is gone); the app
   passes `ManagedBy::Service` to `SyncRuntime::new` to match. A CLI one-shot never starts
   the daemon (Deviation 3), so "started by a client" is no longer a state at all.
5. **`init`/`map` are the framework's `workspace` verbs; the app has no init.** The
   framework's `WorkspaceCommand::{Init { dir: Option<PathBuf>, sync: bool }, List, Map}`
   covers the whole surface (command-system decisions 1/6/7): `workspace init [dir]
   [--sync]` runs **on the app server over IPC** (the server's `workspace.init` handler
   creates the `.sapphire-sync/` marker idempotently and the marker-`config.toml` registry
   entry; `--sync` rides the same connection's `sync.enable`, which mints the `sync-id`
   GrainId and publishes the workspace into the workgroup ledger), and `workspace map
   <selector> [dir]` resolves the selector against the bridge ledger and writes the id via
   `sync.map`. The registry id is the root's slugified directory name — a *different*
   identifier from the sync-id; the two-layer naming (registry: dir-slug, sync: GrainId)
   stays as the framework defines it. Two consequences for this app:
   - **No app-side `init` command and no app core init logic.** The old plan's
     `core::init` (marker + template, app-side) is gone; the marker and registry are the
     server's.
   - **The `.sapphireignore` template is dropped** (the framework's `init` does not write
     it, and an app-side hook for it no longer exists; `SyncFilter::load` treats a missing
     ignore file as "no filter", which is correct behaviour). If the template is wanted
     later it belongs in the framework's `workspace.init` handler — a framework-side
     follow-up, not this app's business.
6. **`sync disable` is the app's only own command.** The framework's `WorkspaceCommand` has
   `init | list | map` but no *unmap/disable* verb (the `sync.disable` IPC method exists;
   the CLI verb does not), so the app ships exactly one app-specific subcommand:
   `sync disable` (stop syncing the resolved workspace; files and sync-id stay, re-enabling
   rejoins the same workspace — the `SyncRuntime::disable` semantics). It is a thin
   `sync.disable` IPC call. If the framework later absorbs a `workspace unmap`-style verb
   (a Phase-3-style sweep), this variant is deleted. A workspace is *enabled* by
   `workspace init --sync` (idempotent — on an already-initialised directory it reports
   "already exists" and still runs `sync.enable`).
7. **`status` is one framework-rendered report.** The app does not print its own status:
   the framework's `Status` renders `StatusReport { running, version, pid, managed_by, app }`
   (not running → one line + exit 1; running → the framework rows then `StatusRow` lines).
   The app injects its per-workspace rows via `AppServer::status_rows(Arc<dyn Fn() ->
   Vec<StatusRow> + Send + Sync>)`: one row per synced root — name = root directory name,
   value = `synced as <id>, N peer(s)` / `not synced`, `paused: <reason>` appended when set.
   Mechanically: the closure is sync but `SyncRuntime::roots()`/`status()` are async, so the
   implementation wraps them in `tokio::task::block_in_place` (the app's runtime is
   multi-thread). Rows show on every `status` while the server runs; a not-running server
   skips them (there is no server to ask).
8. **Superseded E2E scenarios.** Old ③ (takeover) is the bridge's own
   wake-on-sync/lock behaviour, tested in the framework (and the harness below keeps
   `wake_on_sync: false` for exactly that reason); old ⑤ (hosting other apps' workspaces)
   and old ⑥ (standalone `sync` one-shot) are removed by the 2026-09-16 note. What this
   app's E2E keeps: ① pair & propagate, ② conflict copy, ③ missing-root guard,
   ④ filtering & size limit.
9. **E2E harness runs in-process, like the framework's own harnesses**
   (`crates/sapphire-framework-server/tests/common/mod.rs`, which this repo's harness models
   itself on): each simulated host is a real in-process `Bridge` (from
   `sapphire-framework-bridge`) plus a real `AppServer`, two hosts joined over one
   `LoopbackNetwork`. Spawning real processes per host does not work with one process-global
   runtime dir (the framework harness's own comment says why: `SAPPHIRE_RUNTIME_DIR` is one
   variable in one environment); per host the harness builds `BridgeDir::at(host_dir)` +
   per-host `Endpoint::in_dir(…, host_run_dir)` pairs instead.

## Global Constraints

- Code, comments, commit messages and tests in **English**; READMEs in English and
  Japanese, cross-linked at the top (the family `CONTRIBUTING.md` rule).
- CI runs `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --
  -D warnings`, `cargo test --all-features --locked`, and the dependency guard: the
  default build's `cargo tree -p sapphire-sync` must contain **no** `fastembed`, `ort`
  or `ort-sys` — the embedding stack, the heavy part of search. `default-features =
  false` already guarantees it; the check is the guard.
  *(Corrected 2026-10-07. The original guard required `sapphire-framework-retrieve`
  itself to be absent, assuming the feature list kept it out. It never did — the
  framework's workspace layer depends on retrieve unconditionally — and retrieve is
  meant to be in every app, a future `sapphire-sync find` included. Without
  `fastembed-embed` it brings only full-text search.)*
- The framework dependency stays `{ git = "…/sapphire-framework", branch =
  "feat/p2p-sync-iroh", default-features = false, … }` until that branch merges and releases
  v0.1.0, then re-pins to the tag — the only allowed change to the dependency line (and it
  touches `Cargo.lock` in the same commit).
- `AppContext` is `&'static` everywhere: `pub static CTX: AppContext =
  AppContext::new("sapphire-sync");` in the core crate; `CTX.init(kind)` is the *first
  statement of `main` after parsing* (first writer wins: `AppKind::Server` for the bare /
  `serve` path, `AppKind::Cli` for one-shot invocations). App name `sapphire-sync` ⇒ marker
  `.sapphire-sync/` and env overrides `SAPPHIRE_SYNC_{CACHE,DATA,CONFIG}_DIR`.
- Exit codes: 0 success; 1 runtime error (any `Err` out of `main`, printed as
  `sapphire-sync: {err}` to stderr — main must do this itself; the scaffold's plain `Result` main printed `Error: {err:?}`) and also
  the framework dispatch's own `Ok(1)` cases (server-not-running); 2 usage error (clap's
  default — do not override).
- Every public item in the core crate carries a doc comment; `#![warn(missing_docs)]` on
  `sapphire-sync-core`.

## File structure

```
cli/                              # package `sapphire-sync` (bin, thin dispatch)
  src/main.rs                     # CTX.init first, clap parse (AppCommand + flattened
                                  # FrameworkCommand), dispatch
  src/server.rs                   # AppServer build: BridgeClient::connect → SyncRuntime::new
                                  # → AppServer::new().sync().status_rows()
  src/sync.rs                     # the one app command (`sync disable`) + the status rows fn
  tests/cli_parse.rs              # the whole command surface parses; bare = serve
  tests/serve.rs                  # serve/status smoke test (in-process server)
  tests/cli_sync.rs               # disable + status rows against a StubBridge-backed server
  tests/e2e/common/mod.rs         # the two-host harness
  tests/e2e/propagate.rs          # scenario ①
  tests/e2e/conflict.rs           # scenario ②
  tests/e2e/missing_root.rs       # scenario ③
  tests/e2e/filtering.rs          # scenario ④
crates/sapphire-sync-core/
  src/lib.rs                      # #![warn(missing_docs)], pub use framework, CTX
  tests/context.rs                # CTX/env test (EnvGuard pattern)
```

---

### Task 1: Core crate — context and re-exports

**Files:**
- Modify: `crates/sapphire-sync-core/src/lib.rs`
- Test: `crates/sapphire-sync-core/tests/context.rs`

**Interfaces:**
- Consumes: `sapphire_framework::workspace::{AppContext, AppKind}` (and the facade rides
  along as a re-export; Task 1 does *not* yet need the `bridge` feature — Task 2's Step 0
  adds it so this task's diff stays reviewable).
- Produces:
  - `sapphire_sync_core::CTX: AppContext` — `pub static CTX: AppContext = AppContext::new("sapphire-sync");`
  - `sapphire_sync_core` re-exports: `pub use sapphire_framework as framework;` (the CLI
    depends on the core crate only; the facade rides along).
  - No `init.rs`: the marker + registry are the framework server's `workspace.init`
    handler's (Deviation 5).

- [ ] **Step 1: Write the failing test.** `tests/context.rs`, env-guarded exactly like the
      framework's `sync/methods.rs` tests (a `Mutex` serialising the env, an `EnvGuard`
      restoring `SAPPHIRE_SYNC_{CACHE,DATA,CONFIG}_DIR` on drop — copy that pattern,
      `unsafe` set/remove under the one lock). One test: with the three env vars pointing at
      a fresh `tempfile::TempDir` tree, `CTX.init(AppKind::Cli)` resolves `cache_dir`,
      `data_dir` and `config_dir` to those directories (this is what makes every test binary
      in this repo single-CTX-safe).
- [ ] **Step 2: Run, watch it fail** (`CTX` unresolved).
- [ ] **Step 3: Implement** `CTX` + the facade re-export; passing;
      `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all --check` clean.
- [ ] **Step 4: Commit** — `git add crates/sapphire-sync-core && git commit -m "feat: app context and facade re-export"`.

---

### Task 2: The CLI shell + the server — `serve`, `status`, mounted sync

**Command surface (Deviation 3):** everything except `sync disable` is the framework's.
A bare invocation and `serve` are the same thing (`FrameworkCommand::Serve` is `#[default]`).

**Files:**
- Modify: `cli/Cargo.toml` (add `bridge` to the framework features; add dev-dependency
  `sapphire-framework-server` on the *same git source* with `default-features = false,
  features = ["test-util"]` — the facade does not expose `test-util`, so `StubBridge` comes
  from the crate directly)
- Modify: `cli/src/main.rs`
- Create: `cli/src/server.rs`
- Test: `cli/tests/cli_parse.rs`, `cli/tests/serve.rs`

**Interfaces:**
- Consumes: `CTX` from Task 1; the facade's prelude (`server` feature re-exports
  `AppServer`, `FrameworkCommand`, `WorkspaceHost`, …): `FrameworkCommand`
  (`Serve | Status | Service(ServiceCommand) | Workspace | Workgroup | Device`,
  `dispatch(self, server, version)`), `sapphire_framework_server::{SyncRuntime, StatusRow}`,
  `sapphire_framework_bridge::BridgeClient` (`connect(kind, version)` — connect-only,
  `Error::NotRunning` when the bridge is absent: the daemon refuses to run sync-less, see
  below), `sapphire_framework_ipc::Endpoint`, `sapphire_ipc::ManagedBy`,
  `workspace::{AppContext, AppKind, WorkspaceArgs}`, `service::ServiceCommand` (via the
  flattened `FrameworkCommand`).
- Produces:
  - `cli/src/main.rs` `Cli` — the two-field shape the command-system spec's decision 2
    prescribes:
    ```rust
    #[derive(Parser)]
    #[command(name = "sapphire-sync", version, about = "…")]
    struct Cli {
        /// The app's own verbs (Deviation 6: just `sync`).
        #[command(subcommand)]
        app: Option<AppCommand>,
        /// serve / status / service / workspace / workgroup / device.
        #[command(flatten)]
        framework: FrameworkCommand,
        #[command(flatten)]
        workspace: WorkspaceArgs,           // global --workspace-dir
    }
    #[derive(Subcommand)]
    enum AppCommand {
        /// Workspace sync controls: `sync disable`.
        #[command(subcommand)]
        Sync(SyncCommand),
    }
    ```
    `main`: parse; `CTX.init(if matches!(command, Serve|None) { Server } else { Cli })`
    as the first statement; build the bridge-less one-shot `AppServer` (`server::build_oneshot()` —
    the bridge connection belongs to the serve path only, so one-shot verbs never touch the
    bridge); match —
    `None | Some(App)`… no: a *bare* invocation parses to `app: None` **and**
    `framework: FrameworkCommand::Serve` (the default), so the dispatch is
    `framework.dispatch(server, env!("CARGO_PKG_VERSION"))` for everything the framework
    owns, and `sync::dispatch(...)` only for `AppCommand::Sync(_)`; exit-code handling as
    main implements it (any `Err` → eprintln `sapphire-sync: {err}` + exit 1 — the scaffold's plain `Result` main did NOT do this).
  - `cli/src/server.rs`:
    ```rust
    /// Build the serve-path server: the bridge connection (connect-only — a host without a
    /// running bridge cannot sync, so `serve` fails with the bridge's own
    /// "no sapphire-bridge is running" message rather than pretending), the
    /// `SyncRuntime` (`new(&CTX, bridge, current_exe, ManagedBy::Service)` — the daemon
    /// is always service-managed), and the app's status rows (Task 3).
    pub async fn build_serve() -> anyhow::Result<AppServer>;
    /// The bridge-less server one-shot framework verbs dispatch against.
    pub fn build_oneshot() -> AppServer;
    ```
    `build_serve()` = `BridgeClient::connect(CTX.app_name(), VERSION).await` →
    `Arc::new(SyncRuntime::new(&CTX, bridge, std::env::current_exe()?, ManagedBy::Service))`
    → `AppServer::new(&CTX, VERSION).sync(arc)` (+ `.status_rows(...)` from Task 3).
    `serve` runs it: `FrameworkCommand::Serve` → `server.run().await` — SIGTERM/SIGINT and
    the IPC stop channel are handled inside `run()` (the app installs nothing).
- The serve smoke test also proves the two things the framework guarantees: a bare
  invocation *parses* to `Serve`, and `status` against a running in-process server prints
  `running: true` + pid and exits 0 (the app rows themselves are Task 3's test).

- [ ] **Step 0:** add `bridge` to the framework features and the `sapphire-framework-server`
      dev-dependency (same git source, branch pin, `default-features = false`,
      `features = ["test-util"]`); `cargo check` passes.
- [ ] **Step 1: Write the failing parse test** `cli/tests/cli_parse.rs` (export `Cli`/
      `AppCommand`/`SyncCommand` from the bin crate; modelled on the framework's
      `command::tests::the_subcommands_parse`): bare `["sapphire-sync"]` → `app: None` +
      `framework` is `Serve`; `["sapphire-sync", "serve"]`; `"status"`; `"service", "install",
      "--user"`; `"service", "uninstall"`; `"service", "status"`; `"workspace", "init"`,
      `"workspace", "init", "notes"`, `"workspace", "init", "--sync"`; `"workspace", "list"`;
      `"workspace", "map", "notes", "/srv/notes"`; `"workgroup", "list"`; `"device", "list"`;
      `"sync", "disable"`; global `--workspace-dir /tmp/x` everywhere; and a parse collision
      check that the app's `sync` and the framework's `workspace` groups coexist.
- [ ] **Step 2: Watch it fail** (`cargo test -p sapphire-sync --test cli_parse`).
- [ ] **Step 3: Implement** `main.rs` (shape above; the `Sync` arm dispatches to Task 3's
      `sync::dispatch` — land Task 3 first or stub its arm) and `server.rs` (as above).
      One smoke integration test `cli/tests/serve.rs`: set `SAPPHIRE_SYNC_{CACHE,DATA,
      CONFIG}_DIR` and `SAPPHIRE_RUNTIME_DIR` to a temp tree (env-guard pattern; the
      framework's `endpoint.rs` `EnvGuard`/`run_with_env` shows the shape), build the server
      with `.endpoint(Endpoint::for_app("sapphire-sync")?)` implied by the env (no explicit
      endpoint), spawn `server.run()` in a task, wait until `sapphire_ipc::probe` says the
      socket is listening (poll with a deadline — a fast first probe is not a failure), then
      run `FrameworkCommand::Status.dispatch(...)` in-process and assert exit 0 + output
      contains `running: true`, then tear the server down (abort the task).
- [ ] **Step 4: All green; fmt + clippy clean; commit** —
      `"feat: framework command surface (serve, status, service) + mounted SyncRuntime"`.

---

### Task 3: `sync disable` + the app's status rows

**Command surface (Deviations 5–7):** enabling/mapping/listing are the framework's
`workspace` verbs; only *disabling* is the app's own verb. Status is the framework's
report; the app injects rows.

**Files:**
- Create: `cli/src/sync.rs`
- Modify: `cli/src/main.rs` (the `Sync` arm) and `cli/src/server.rs` (attach `status_rows`)
- Test: `cli/tests/cli_sync.rs`

**Interfaces:**
- Consumes: `IpcBackend::connect(&Endpoint::for_app("sapphire-sync")?, "sapphire-sync",
  "cli", VERSION, root)` — connect-only, never starts anything — then
  `backend.client().call::<_, Ack>(SYNC_DISABLE, WsParams { ws })`;
  `Workspace::find_from(&CTX, …)`/`resolve` (the framework's own root resolution, honoring
  `--workspace-dir` and the upward `.sapphire-sync/` search — the same resolution the
  server's `workspace.init` uses server-side); `SyncRuntime::roots()` +
  `SyncRuntime::status(&root) -> SyncStatus { enabled, workspace_id, peers, paused,
  last_error, bridge_available }` for the rows;
  `AppServer::status_rows(Arc<dyn Fn() -> Vec<StatusRow> + Send + Sync>)`.
- Produces:
  ```rust
  pub enum SyncCommand {
      /// Stop syncing the workspace this command resolves to. Files and sync-id stay;
      /// re-enabling via `workspace init --sync` rejoins the same workspace.
      Disable,
  }
  pub async fn dispatch(command: SyncCommand, root: Option<PathBuf>) -> anyhow::Result<i32>;
  /// One row per synced workspace, for `AppServer::status_rows` (Deviation 7:
  /// `block_in_place` around the async runtime calls).
  pub fn status_rows(runtime: Arc<SyncRuntime>) -> Arc<dyn Fn() -> Vec<StatusRow> + Send + Sync>;
  ```
  `dispatch` resolves the root, connects (`connect_or_absent` semantics: no server → print
  + exit 1 — the framework error text already says "no sapphire-sync server is running"),
  calls `sync.disable`, prints `sync disabled for <root>` and returns 0. Success is exit 0
  whether or not the root was synced (`SyncRuntime::disable` is idempotent). Any other
  `Err` propagates to `main` → exit 1.

- [ ] **Step 1: Write the failing tests.** `cli_parse.rs` already covers parse.
      `cli_sync.rs`: stand up one in-process host per Task 4's harness shape *minus the
      bridge* (an `AppServer` with a `SyncRuntime` over `StubBridge::start()` —
      `sapphire_framework_server::sync::testing::StubBridge`, which yields
      `(StubBridge, Arc<BridgeClient>)` over a `Connection::pair`); enable a temp workspace
      via `runtime.enable(&root)`, then: the status-rows fn returns one row containing the
      id and "1 peer"… (stub answers `peers` with the stub + this host — assert exactly the
      row text the implementation prints, `synced as <id>, 0 peers` style: the stub's
      single-node ledger says `peers = 1` ⇒ the row says `0 peers` per the framework's
      saturating_sub(1) convention — assert what the code says); `disable` via
      `sync::dispatch` (against the in-process server: dispatch through the *real*
      `FrameworkCommand` path where feasible, else call `dispatch` against a client built on
      `connect_or_absent` to the test endpoint) → the stub's `unregistrations` grow and the
      row flips to "not synced"; re-enable via the `sync.enable` method path the framework's
      `workspace init --sync` rides → idempotent, no error.
- [ ] **Step 2: Watch them fail.**
- [ ] **Step 3: Implement** `sync.rs` + the `main.rs` arm + the `status_rows` wiring; passing.
- [ ] **Step 4: fmt/clippy/tests; commit** —
      `"feat: sync disable + app status rows"`.

---

### Task 4: E2E harness + scenario ① (pair and propagate)

**Files:**
- Create: `cli/tests/e2e/common/mod.rs`
- Test: `cli/tests/e2e/propagate.rs`

**What the harness is (Deviation 9):** model it on the framework's
`crates/sapphire-framework-server/tests/common/mod.rs` (read it first — it already solves
"two hosts, one process": one `LoopbackNetwork` shared by two `Bridge`s each with its own
`BridgeDir::at(host_dir)` and per-host `Endpoint::in_dir(…, host_run_dir)` control/data
endpoints, one shared `static CTX: AppContext` whose per-test scratch tree is set via
`CTX.set_cache_dir/set_data_dir/set_config_dir` *before* the first init — first writer
wins). Per host: one `Bridge::new(dir, net.transport(node_id), VERSION)` with
`wake_on_sync: false` (the exe a route names would be the test binary), one `AppServer`
(built exactly like Task 2's `build()` but per host, each with its own `BridgeClient` via
`BridgeClient::from_client(connect_or_absent(...).1, run_dir)` and its own
`SyncRuntime::new(CTX, bridge, current_exe, ManagedBy::Service)`), one workspace root
(`tmp/host{a,b}/notes/` with the `.sapphire-sync/` marker + the same `sync-id` file content
on both hosts — the harness writes the id file directly rather than running the CLI; the
registry `config.toml` entry is *not* needed for sync (the sync-id is the sync identity;
the registry is the CLI's local list) — **record the verified finding in the harness doc
comment**, since Tasks 5 and later readers rely on it). Cache-key note to verify and record:
each workspace's cache is `<cache>/<uuid>/` keyed off the *root path*, so two hosts with
different root paths never collide on one shared tree. One workgroup: `Workgroup::create`
once on host A's dir with a fixed `NODE_A`, host B joins by copying the ledger like the
framework's `adopt_workgroup` fixture; then `SyncRuntime::enable` on both hosts (`enable`'s
dial makes the first session immediate). Helpers: `write(host, rel, content)`,
`poll_until(|| condition, "what")` (10 s timeout, 25 ms poll — copy from the framework
harness; **no sleeps without a condition**), `settle`/`quiet` (copy their
deadline-and-poll shape: they wait for *quiet*, never a fixed window).

**Scenario ①** (`propagate.rs`): enable on both hosts → write a file on A → it appears
under B's root (poll); modify on B → A converges; delete on A → gone from B. Then the same
file tree on both sides is the assertion (file-by-file compare of the two roots).

- [ ] **Step 1:** write the harness + `propagate.rs` (the scenario above, three `poll_until`s).
- [ ] **Step 2:** run, watch it fail to compile, then fail the first poll.
- [ ] **Step 3:** implement the harness until the scenario passes:
      `cargo test -p sapphire-sync --test propagate` (each test file is its own crate;
      include the harness with `#[path = "e2e/common/mod.rs"] mod common;` or mirror however
      the framework's `converge.rs` includes `common/`).
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
- **Missing-root guard:** rename a synced root away on A → A's `SyncRuntime::status` reports
  `paused: Some("RootMissing")` (the `PauseReason` Debug form — asserting on the runtime
  directly is fine here; the CLI row just prints it); the peer's files are untouched;
  rename back; `paused` clears and a new file written on B reaches A.
- **Filtering & limits:** a `.sapphireignore` (created *before* enable on A; it syncs with
  everything else — assert it exists on B too; `.sapphireignore.conflict-*` copies stay
  excluded by the built-in rule) excluding `*.tmp` → an `x.tmp` written on A never reaches
  B; a file larger than `sapphire_framework::sync::DEFAULT_MAX_FILE_SIZE` (64 MiB,
  re-exported at the sync crate's root; the `replica` module itself is private) is skipped
  (absent on B) while its smaller sibling syncs; oversized files are *silent* skips — the
  file-level assertions are the real checks.

- [ ] **Steps per file:** write → watch fail → implement the harness helpers the scenario
      still needs (rename-root helper, oversized payload builder: `DEFAULT_MAX_FILE_SIZE + 1`
      bytes) → passing → fmt/clippy.
- [ ] **Commit** — `"test: conflict, missing-root and filtering e2e scenarios"`.

---

### Task 6: READMEs, CI, CONTRIBUTING

**Files:** `README.md`, `README.ja.md`, `.github/workflows/ci.yml`, `CONTRIBUTING.md`.

- [ ] **README (en + ja, cross-linked at the top).** Sections per spec §4 *as revised*:
      quick start (`sapphire-bridge workgroup create --device-name laptop home` and
      `sapphire-sync workspace init --sync ~/Documents/notes` → the workspace shows in
      `workspace list` on every device and two-way sync begins; the bridge's own verbs are
      its README's business — link, do not restate); how the pieces fit (this app = the app
      server + CLI, the `sapphire-bridge` app = the always-on peer and switchboard; one
      `sapphire-sync serve` per host, installed by `service install`, stopped by the
      service manager); `status` output (the framework's rows + this app's per-workspace
      rows); `.sapphireignore`; how conflicts appear (the `.conflict-…` copies and why both
      versions survive); the "no server is running" contract (one-shot commands never start
      the daemon).
- [ ] **CI:** the four CI commands from Global Constraints as one workflow
      (ubuntu + windows matrix; the dependency guard greps the full `cargo tree -p
      sapphire-sync` for `fastembed`/`ort`/`ort-sys` and asserts none — not `cargo tree
      -i`, which errors when the crate is absent).
- [ ] **CONTRIBUTING.md:** this repository's rule (two-crate layout `cli/` +
      `crates/sapphire-sync-core/`, the thin-CLI rule — everything the framework provides
      stays the framework's, tests live in `cli/tests/`); link the framework repo's
      `CONTRIBUTING.md` for the family-wide rules instead of copying them.
- [ ] **Commit** — `"docs: READMEs (en/ja), CI, contributing"`.

---

## Out of scope (unchanged from spec, incl. superseded parts)

Send-only/receive-only folders, file versioning, a web UI, a desktop/tray app (a future
separate crate), binary release artifacts. Workgroup/device/network commands live in
`sapphire-bridge` (and reach this app's CLI through the framework's `workgroup`/`device`
groups, which only forward to the bridge — superseding note, bullet 3). Service
start/stop/restart abstractions (framework Non-goal), privilege separation config, and any
per-app status *format* changes (the `StatusReport` shape is the framework's to change).
