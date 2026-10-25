# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.1](https://github.com/sapphire-turtle/sapphire-sync/compare/sapphire-sync-core-v0.14.0...sapphire-sync-core-v0.14.1) - 2026-10-25

### Other

- update Cargo.toml dependencies

## [0.14.0](https://github.com/sapphire-turtle/sapphire-sync/compare/sapphire-sync-core-v0.13.0...sapphire-sync-core-v0.14.0) - 2026-10-15

### Other

- *(license)* per-component license files; desktop under GPL-3.0-or-later

## [0.13.0](https://github.com/sapphire-turtle/sapphire-sync/releases/tag/sapphire-sync-core-v0.13.0) - 2026-10-13

### Added

- framework command surface (serve, status, service) + mounted SyncRuntime
- app context and facade re-export

### Other

- *(release)* set up release-plz for the first release at 0.13.0
- *(deps)* drop the unused tracing dependency from the CLI and core
- describe the embedding-stack guard, not a retrieve-free graph
- workspace scaffold (root workspace, core lib, server-embedded CLI skeleton)
