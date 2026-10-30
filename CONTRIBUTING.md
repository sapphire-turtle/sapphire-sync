# Contributing

This repository's own rules. Family-wide conventions — commit style, the
doc-comment rule, the English-code convention — live in the
[sapphire-framework CONTRIBUTING.md](https://github.com/sapphire-turtle/sapphire-framework/blob/main/CONTRIBUTING.md).
Link, don't copy.

## Repository layout

Two editions, each one binary, over the same libraries:

- `headless/` — `sapphire-sync-headless`, the `sapphire-sync` binary without
  the GUI: a `main` that runs `sapphire-sync-cli`.
- `desktop/` — `sapphire-sync-desktop`, the GUI, which runs the same commands
  when given one.
- `crates/sapphire-sync-cli/` — the command line both editions run: a thin
  clap shell plus the app's own `sync` verb. Everything the framework provides
  stays the framework's; this repository re-exports, never re-implements.
- `crates/sapphire-sync-server/` — the app server `serve` runs, and the status
  rows it reports.
- `crates/sapphire-sync-core/` — the core crate: the `CTX` static and the
  framework re-export. Keep it thin; it exists so the app has something of
  its own to depend on.

## Tests

Each crate's tests live in its own `tests/`: the command line's parse and
`sync` tests in `crates/sapphire-sync-cli/tests/`, the two-host sync scenarios
in `crates/sapphire-sync-server/tests/`, and the tests that run the real
`sapphire-sync` binary in `headless/tests/`. Top-level `tests/*.rs` files are
separate test crates; `crates/sapphire-sync-server/tests/e2e/common/` is a
shared harness included by path, not a crate of its own.

## CI

`.github/workflows/ci.yml` calls the organization's shared Rust CI
([sapphire-turtle/.github](https://github.com/sapphire-turtle/.github/blob/main/README.md)):
`cargo fmt --check`, then `cargo clippy -D warnings` and `cargo test --locked`
over every crate but the desktop app, on Linux x86_64 (`ci / test`) and on Linux
aarch64, macOS and Windows (`ci / test-platforms`). On Linux x86_64 it then runs
a dependency guard — the default build's `cargo tree -p sapphire-sync-headless` must not
contain `fastembed`, `ort` or `ort-sys` (the embedding stack, the heavy part of
search). `sapphire-framework-retrieve` itself is expected: the framework's
workspace layer depends on it in every app, and without `fastembed-embed` it
brings only full-text search.

## What belongs here

This app is the reference: it exists to prove the framework's public surface
is enough. If a change here needs a new framework API, that is a framework
issue, not an app change — the app should stay as small as the framework
allows.
