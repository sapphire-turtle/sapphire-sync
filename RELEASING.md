# Releasing

Releases for this repository are automated with [release-plz](https://release-plz.dev/).

## Normal flow

1. When changes are pushed to `main`, release-plz creates or updates a release PR.
2. Review the release PR (versions, each crate's `CHANGELOG.md`, `Cargo.toml`).
3. Merging the PR publishes the crates to crates.io and creates the tags and GitHub releases, then builds the binaries (Linux x86_64 / aarch64, macOS aarch64, Windows x86_64) and attaches them to those releases. To (re)upload them for an existing tag, run "Release binaries" manually with that tag and `cli` or `desktop`. The build itself is sapphire-framework's shared `shared-release-binary.yml`.

Nothing is published by a push that is not a release PR's merge (`release_always = false`), even when `Cargo.toml` carries a version crates.io does not have yet.

## What is versioned how

| Crate | Version | Published | Tag / GitHub release |
| --- | --- | --- | --- |
| `sapphire-sync` (CLI and server) | `workspace.package.version` | crates.io | `v{version}` |
| `sapphire-sync-core` | `workspace.package.version` | crates.io | — |
| `sapphire-sync-desktop` | its own | GitHub release only | `desktop-v{version}` |
| `sapphire-sync-api` | its own | crates.io | — |

`sapphire-sync` and `sapphire-sync-core` are one version group: they always move together.

`sapphire-sync-desktop` is versioned on its own, so a change to the GUI alone releases only the desktop app. It is not published to crates.io, so release-plz reads its current version from its `desktop-v{version}` tags (`git_only = true`); without `git_only`, release-plz skips a `publish = false` crate without a word.

`sapphire-sync-api` is the server's own API. Its major version is the `app_api` the server reports in the IPC handshake (`API_VERSION`, parsed from the crate version at compile time), so it moves only when that API changes, the way `sapphire-framework-bridge-api` and `sapphire-framework-server-api` do. A breaking change to its methods or types is a major release of it (mark the commit with `!`); a new method is a minor release. A sapphire-sync release that leaves it alone does not demand that every client and server on a host be replaced together.

The crates.io name `sapphire-sync` belonged to the git sync library of sapphire-workspace, whose last release is 0.12.1, so this app's first release is 0.13.0.

## Version bump rules

release-plz infers the version from conventional commits (assuming pre-1.0 `0.x.y` for the app's crates).

- `feat:` / `fix:` / `chore(deps):` and other non-breaking changes → patch bump
- `feat!:` / `chore!:` or a `BREAKING CHANGE:` footer → minor bump

Moving to a sapphire-framework release whose server API (`sapphire-framework-server-api`) or bridge API (`sapphire-framework-bridge-api`) has a new major is breaking for the app as well — the CLI, the server and the bridge on a host have to be replaced together — so mark that commit with `!`.

## Required secrets

- `RELEASE_PLZ_TOKEN`: fine-grained PAT with Contents + Pull requests read/write. A PAT rather than `GITHUB_TOKEN`, so that CI runs on the release PR and the PR can be opened at all (the repository does not let Actions create pull requests).
- `CARGO_REGISTRY_TOKEN`: used for crates.io publishing. It must be able to publish `sapphire-sync`, which already exists on crates.io under `fluo10`, as well as the new `sapphire-sync-core` and `sapphire-sync-api`.
