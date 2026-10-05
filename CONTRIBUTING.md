# Contributing

This repository's own rules. Family-wide conventions — commit style, the
doc-comment rule, the English-code convention — live in the
[sapphire-framework CONTRIBUTING.md](https://github.com/fluo10/sapphire-framework/blob/main/CONTRIBUTING.md).
Link, don't copy.

## Repository layout

Two crates, one published binary:

- `cli/` — the `sapphire-sync` binary: a thin clap shell plus the app's own
  `sync` verb. Everything the framework provides stays the framework's; this
  repository re-exports, never re-implements.
- `crates/sapphire-sync-core/` — the core crate: the `CTX` static and the
  framework re-export. Keep it thin; it exists so the binary has something of
  its own to depend on.

## Tests

Tests live in `cli/tests/`. Top-level `tests/*.rs` files are separate test
crates; `cli/tests/e2e/common/` is a shared harness included by path, not a
crate of its own.

## CI

`.github/workflows/ci.yml` runs an ubuntu + windows matrix over four steps:
`cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --
-D warnings`, `cargo test --all-features --locked`, and a dependency guard —
`cargo tree -p sapphire-sync -i sapphire-framework-retrieve` must print
nothing (a sync-only app must not pull in the search stack).

## What belongs here

This app is the reference: it exists to prove the framework's public surface
is enough. If a change here needs a new framework API, that is a framework
issue, not an app change — the app should stay as small as the framework
allows.
