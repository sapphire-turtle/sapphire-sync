# Releasing

Releases for this repository are automated with [release-plz](https://release-plz.dev/).

## Normal flow

1. When changes are pushed to `main`, release-plz creates or updates a release PR.
2. Review the release PR (versions, each crate's `CHANGELOG.md`, `Cargo.toml`).
3. Merging the PR publishes the crates to crates.io and creates the tags and GitHub releases, then builds the binaries (Linux x86_64 / aarch64, macOS aarch64, Windows x86_64) and attaches them to those releases. Both editions' binaries go on the one `v{version}` release, as `sapphire-sync-headless-<os>-<arch>` and `sapphire-sync-desktop-<os>-<arch>`. To (re)upload them for an existing tag, run "Release binaries" manually with that tag. The build itself is sapphire-turtle/.github's shared `release-binary.yml`.

Nothing is published by a push that is not a release PR's merge (`release_always = false`), even when `Cargo.toml` carries a version crates.io does not have yet.

## What is versioned how

| Crate | Version | Published | Tag / GitHub release |
| --- | --- | --- | --- |
| `sapphire-sync-headless` (the `sapphire-sync` binary, no GUI) | `workspace.package.version` | crates.io | `v{version}` |
| `sapphire-sync-desktop` (the GUI, which runs the same commands) | `workspace.package.version` | GitHub release only | `v{version}` (the headless edition's) |
| `sapphire-sync-cli` (the command line both run) | `workspace.package.version` | crates.io | — |
| `sapphire-sync-server` (the app server) | `workspace.package.version` | crates.io | — |
| `sapphire-sync-core` | its own | crates.io | — |
| `sapphire-sync-api` | its own | crates.io | — |

The two editions and the two libraries they run are one version group (`app`): they always move together, and one `v{version}` tag and GitHub release, made by `sapphire-sync-headless`, carries both editions' binaries. The desktop app's binary runs every command of the headless one, so the two must not drift apart.

`sapphire-sync-desktop` is not published to crates.io, so release-plz reads its current version from the `v{version}` tags (`git_only = true`, with its own tag and release turned off); without `git_only`, release-plz skips a `publish = false` crate without a word. Until 0.14.1 it was versioned on its own, under `desktop-v{version}` tags.

`sapphire-sync-core` is versioned on its own: it holds the `CTX` static and the framework re-export, and moves only when they change.

`sapphire-sync-api` is the server's own API. Its major version is the `app_api` the server reports in the IPC handshake (`API_VERSION`, parsed from the crate version at compile time), so it moves only when that API changes, the way `sapphire-framework-bridge-api` and `sapphire-framework-server-api` do. A breaking change to its methods or types is a major release of it (mark the commit with `!`); a new method is a minor release. A sapphire-sync release that leaves it alone does not demand that every client and server on a host be replaced together.

The crates.io name `sapphire-sync` belonged to the git sync library of sapphire-workspace, whose last release is 0.12.1, so this app's first release was 0.13.0. 0.13.0 to 0.14.1 were published as `sapphire-sync`; from 0.15.0 the binary is `sapphire-sync-headless`, beside the `sapphire-sync-cli` and `sapphire-sync-server` libraries, all three new crates.io names. New names are rate-limited on crates.io (about one every ten minutes), so the first release that publishes them may need its job re-run.

## Version bump rules

release-plz infers the version from conventional commits (assuming pre-1.0 `0.x.y` for the app's crates).

- `feat:` / `fix:` / `chore(deps):` and other non-breaking changes → patch bump
- `feat!:` / `chore!:` or a `BREAKING CHANGE:` footer → minor bump

Moving to a sapphire-framework release whose server API (`sapphire-framework-server-api`) or bridge API (`sapphire-framework-bridge-api`) has a new major is breaking for the app as well — the CLI, the server and the bridge on a host have to be replaced together — so mark that commit with `!`.

## Required secrets

- `RELEASE_PLZ_TOKEN`: fine-grained PAT with Contents + Pull requests read/write. A PAT rather than `GITHUB_TOKEN`, so that CI runs on the release PR and the PR can be opened at all (the repository does not let Actions create pull requests).
- `CARGO_REGISTRY_TOKEN`: used for crates.io publishing. It must be able to publish `sapphire-sync-core` and `sapphire-sync-api`, which already exist on crates.io under `fluo10`, as well as the new `sapphire-sync-headless`, `sapphire-sync-cli` and `sapphire-sync-server`.
