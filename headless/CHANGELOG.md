# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.1](https://github.com/sapphire-turtle/sapphire-sync/compare/v0.14.0...v0.14.1) - 2026-10-25

### Added

- let cargo binstall install the release binaries

### Other

- update to sapphire-framework 0.22
- *(desktop)* start through sapphire-framework's gui::app::run

### Changed

- [**breaking**] sapphire-framework 0.22.0. `serve` restarts in place on `server.restart`, the server's settings move from `server.toml` to `config.toml` (migrated on the first start), `config list|get|set|unset` reads and writes the settings, and `workgroup`, `device`, `embedding`, `external-device` and `workspace show` go through the running server. Upgrade `sapphire-bridge` together with this release: an older bridge fails the handshake.

## [0.14.0](https://github.com/sapphire-turtle/sapphire-sync/compare/v0.13.0...v0.14.0) - 2026-10-15

### Added

- *(cli)* [**breaking**] sapphire-framework 0.20.0; print the help on a bare invocation
- *(cli)* serve before the bridge is up, and reconnect after it restarts

### Fixed

- *(cli)* [**breaking**] drop the deprecated global --workspace-dir

### Other

- Merge pull request #16 from sapphire-turtle/chore/framework-0.19
- *(license)* per-component license files; desktop under GPL-3.0-or-later

## [0.13.0](https://github.com/sapphire-turtle/sapphire-sync/compare/v0.12.1...v0.13.0) - 2026-10-13

### Added

- *(api)* split out the server's own API as sapphire-sync-api
- sync disable + app status rows
- framework command surface (serve, status, service) + mounted SyncRuntime

### Fixed

- *(cli)* drop the `--workspace-dir` guard from `sync disable`
- *(cli)* run `sync disable` on the server's workspace, not the cwd's
- *(cli)* drop the `not synced` status row
- *(cli)* mount sync on an explicit `serve`
- *(cli)* send the API version in the IPC handshake
- error format, bridge-less one-shot dispatch, env-guard capture, test assertions

### Other

- *(release)* set up release-plz for the first release at 0.13.0
- *(deps)* move to sapphire-framework 0.18.0
- *(deps)* move to sapphire-framework 0.16.0
- *(deps)* move to sapphire-framework 0.15.0 from crates.io
- *(deps)* drop the unused tracing dependency from the CLI and core
- address task 5 review minors (helper locality, test naming, honest comments)
- conflict, missing-root and filtering e2e scenarios
- remove probe-era diagnostics and unused deps from the e2e harness
- two-host e2e harness and propagate scenario (Task 4)
- workspace scaffold (root workspace, core lib, server-embedded CLI skeleton)
