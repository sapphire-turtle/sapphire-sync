# SDD ledger — plan: docs/superpowers/plans/2026-09-23-sync-app-plan.md

Plan revised 2026-09-26 (commit f95b109) for the merged framework command system
(#146/#150, branch tip 5780e0c). Branch: feat/sync-app, base f95b109.

## Pre-flight scan

| Pair/Task | Check | Finding | Ruling |
|---|---|---|---|
| T1↔T2/T3 | CTX/re-export produced by T1 consumed by T2/T3 | Consistent (CTX static, init first statement, env vars SAPPHIRE_SYNC_*) | none |
| T2↔T3 | server.rs build() consumed by T3's tests & status_rows wiring | Consistent: T2 produces build() w/o status_rows; T3 adds it | none |
| T3↔T4 | harness shape: T3's StubBridge host vs T4's two-host LoopbackNetwork harness | Different fixtures on purpose (single-host StubBridge for CLI tests; two-host for E2E); plan says so | none |
| T2 internal | `Cli` shape: bare parse = app:None + framework:Serve(default) | Matches framework FrameworkCommand (Serve #[default]); dispatch = framework.dispatch(server, version) | none |
| scaffold | cli/Cargo.toml lacks `bridge` feature | Plan Task 2 Step 0 adds it + framework-server test-util dev-dep | planned, no conflict |
| plan text | old plan's `sync enable/map` CLI verbs | Revised plan Deviation 5/6: only `sync disable` is app-owned; enable via `workspace init --sync` (framework) | already ruled in the revised plan |

No load-bearing conflicts found. Specs: framework repo docs/superpowers/specs/2026-09-15-sapphire-sync-design.md (as superseded), 2026-09-16-process-architecture-design.md, 2026-09-24-app-command-system-design.md.

## Task 1: complete (commits f95b109..189f8b6, review clean)
- Task 1: minor (deferred): lib.rs:16 broken intra-doc link `[`CTX::init`]` (rustdoc warning; fix alongside Task 2's cli/ fmt pass)
- Note: scaffold cli/src/main.rs has a pre-existing fmt issue; Task 2 touches cli/ and must run cargo fmt --all (or -p) clean.

## Task 2: review round 1 -> fix dispatch
- Task 2: Ruling (review Important #4): build() connects the bridge only on the serve path; one-shot framework verbs dispatch against a bridge-less AppServer (dispatch needs only app_name/service_spec; matches framework's own status_tests). The plan's "building it for a one-shot is cheap" premise corrected: a bridge-connecting build() is NOT cheap/valid for one-shots.
- Task 2 review findings sent to fix round 1: (1) main must print `sapphire-sync: {err}` + exit 1 (plan's scaffold claim was false; fixed plan text), (2) serve.rs must assert output contains running: true (+pid) — subprocess CARGO_BIN_EXE or captured output, (3) Env::set captures previous values AFTER set_var (restore no-op) — capture before, (4) ruling above.
- Task 2: minor (deferred): help-text jargon (Deviation 6 in --help), None-arm discards exit code, sync placeholder returns Ok(()) semantics, unused serde_json dev-dep, stale main.rs comment, duplicated bare-parse test, ManagedBy dual path (resolved by ruling: single source), bridge feature adds build weight until Task 4 (brief-sanctioned).

## Task 2: fix round 1/5 (5 addressed, 0 open; commits 0acb194..1e82e6b)
- Task 2: complete (commits 189f8b6..1e82e6b, review clean after round 1)
- Task 2: ruling applied: bridge connection serve-path-only (build_oneshot bridge-less; plan text corrected to say so)

## Task 3: complete (commit d52473c, review Approved — no fix round)
- Task 3: ruling applied: absent-server `sync disable` prints framework's one-line
  `no sapphire-sync server is running` on stdout + exit 1 (app prints via connect_or_absent
  semantics; text verbatim same as framework's run_status_into); other errors -> main stderr format.
- Task 3: ruling applied: status_rows = runtime roots() (sorted) via status() + one extra
  `not synced` row for the cwd-resolved workspace when not in roots; block_in_place accepted
  (multithread runtime always; framework calls closure from multi-thread runtime - verified).
- Task 3: implementer hit file_write mangling twice; heredoc workaround, committed content
  verified line-by-line (reviewer confirmed no mangling).
- Task 3: minors deferred: (1) cli_sync.rs duplicates ~60 lines of serve.rs test scaffolding
  (Env/leaf_of/ENV_LOCK/wait_until_listening) - hoist to tests/common/mod.rs if a 3rd test
  binary arrives (Task 4 adds e2e/ - consider then); (2) serve.rs comment backtick churn in diff;
  (3) report line-count cosmetic.

## Task 4: attempt 1 (implementer) exhausted tool budget with no written artifacts
- Tree clean at d52473c. Resume with handoff: framework test-util APIs already surveyed once
  (testing.rs LoopbackNetwork/BridgeDir/Bridge::new wake_on_sync:false; framework-server
  tests/common/mod.rs two-host harness; converge.rs poll_until/settle/quiet shapes;
  find_from/marker/sync-id in workspace.rs; cache <cache>/<uuid>/ keyed off root path).

## Task 4: fix round (cleanup, commit 4d00d26)
- Review verdict was "needs work (small, deletion-only)": probe-era scaffolding left in the harness.
- Removed: per-step HARNESS/STEP eprintln traces in synced_pair/propagate; diagnostic_status();
  the "Controller's decisive-dump helpers" impl block (Host::stop, Host::device_ids).
- Removed unused dev-deps from cli/Cargo.toml + workspace Cargo.toml/lock: sapphire-framework-sync,
  grain-id, uuid, walkdir (harness reaches everything via the facade; walks roots with its own
  stack-based walk).
- Reworded synced_pair doc to state what the harness actually does: each host founds its own
  workgroup and exchanges copies of device records — not one shared workgroup joined by copy.
- Dependency pin updated to b719c5e (#159-#166 merged on feat/p2p-sync-iroh; enable_sync's
  never-Hello session now fails fast instead of parking — the propagate flake's root fix).
- Verification: propagate 4 consecutive PASS (no flakes), cli_parse 9/9, cli_sync 3/3, serve 1/1,
  cargo clippy --all-targets clean, cargo fmt applied to touched files.

## Task 4: complete (commits d52473c..4d00d26, review clean after fix round)
- Re-review verdict: all 4 findings ADDRESSED (probe残骸削除 / synced_pair doc実態相符 / 未使用依存4種削除 / pin b719c5e確認), no new breakage, no open findings.
- Next: Task 5 (E2E scenarios — conflict, missing root, filtering & limits), Task 6 (READMEs, CI, CONTRIBUTING).
