# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.0](https://github.com/sapphire-turtle/sapphire-sync/compare/desktop-v0.13.0...desktop-v0.14.0) - 2026-10-15

### Added

- *(cli)* [**breaking**] sapphire-framework 0.20.0; print the help on a bare invocation
- *(cli)* serve before the bridge is up, and reconnect after it restarts

### Other

- Merge pull request #16 from sapphire-turtle/chore/framework-0.19
- *(license)* per-component license files; desktop under GPL-3.0-or-later

## [0.13.0](https://github.com/sapphire-turtle/sapphire-sync/releases/tag/desktop-v0.13.0) - 2026-10-14

### Added

- *(desktop)* a desktop app around the framework's SyncPanel

### Fixed

- *(release)* release the desktop app, versioned on its own
- *(desktop)* force Vulkan on Windows only; document WGPU_BACKEND=dx12

### Other

- *(release)* set up release-plz for the first release at 0.13.0
- *(deps)* move to sapphire-framework 0.18.0
- *(deps)* move to sapphire-framework 0.16.0
- *(deps)* move to sapphire-framework 0.15.0 from crates.io
- *(deps)* track the framework's integration branch now that the GUI work is merged
