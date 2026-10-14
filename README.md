# sapphire-sync

[English](README.md) | [日本語](README.ja.md)

## What this is

sapphire-sync is a peer-to-peer file-sync application built on
[sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework). It is the
smallest useful app the framework can carry: a sync-only reference app, in the
Syncthing style — a bare invocation of the binary runs the embedded server, and
every other subcommand is a one-shot client that talks to that server over the
local IPC endpoint.

## Install

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- sync
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) sync
```

This installs `sapphire-sync` together with `sapphire-bridge`, which it syncs
through, into `~/.local/bin`. Add `sync-desktop` for the desktop app. See the
[install script's README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install)
for the options.

## Quick start

```console
$ sapphire-bridge serve &            # the always-on peer (sapphire-bridge app)
$ sapphire-sync workspace init --sync ~/Documents/notes
$ sapphire-sync serve &             # this host's sync node
$ sapphire-sync workspace show      # this host's workspace, then the workgroup's
```

The framework's own verbs (`workgroup create`, `device invite`, `workgroup join`)
are documented in the framework repository's README — this app re-exports them
unchanged.

## How the pieces fit

This repository ships one binary: the app server plus the CLI. The always-on
peer it talks to is the `sapphire-bridge` app; run exactly one
`sapphire-sync serve` per host. The server serves one workspace at a time
(`workspace select` switches it); the `SyncRuntime` mounts its replica and
registers it with the bridge; the app itself implements
nothing of the replication engine.

## Status output

`sapphire-sync status` prints the framework's rows (app, version, endpoint) plus
this app's per-workspace rows — one line per synced workspace, rendered from the
live `SyncRuntime`.

## `.sapphireignore`

A `.gitignore`-style file that lives *inside* the workspace and is synced like any
other file. Built-in rule: the ignore file's own conflict copies
(`.sapphireignore.conflict-*`) stay excluded, so conflict noise never propagates.

## Conflicts

Concurrent edits resolve by keeping both versions: the loser is written beside
the winner as `<stem>.conflict-<replica-id>-<counter>.<ext>`. Nothing is
overwritten; conflicts are resolved by hand, the Syncthing way.

## The "no server is running" contract

One-shot verbs never start the daemon. When no server is listening they print
`no sapphire-sync server is running` and exit 1.

## Desktop app

`sapphire-sync-desktop` is a GUI client of the installed services: it shows the
framework's sync panel and never runs a server or bridge itself, so closing the
window does not stop sync. Install it beside `sapphire-sync` and
`sapphire-bridge` (`sh -s -- sync sync-desktop` above), so its "Install & start
service" buttons can find them.

On Windows it renders with Vulkan, because the DX12 device is lost whenever a
Remote Desktop session reconnects; elsewhere wgpu picks its usual backend. Set
`WGPU_BACKEND` to override this. If the app crashes at start on Windows (a
Vulkan overlay layer can do that), run it with `WGPU_BACKEND=dx12`.

## Links

- [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework) — the
  framework this app builds on
- [CONTRIBUTING.md](CONTRIBUTING.md) — how to contribute to this repository

## License

This repository contains components under different licenses:

| Component | License |
|-----------|---------|
| `sapphire-sync` (`cli/`) | MIT OR Apache-2.0 |
| `sapphire-sync-core` | MIT OR Apache-2.0 |
| `sapphire-sync-api` | MIT OR Apache-2.0 |
| `sapphire-sync-desktop` | GPL-3.0-or-later, with [App Store marketplace exception](desktop/LICENSE.App-Store-Exception) |

See the `LICENSE` (or `LICENSE-MIT` / `LICENSE-APACHE`) file in each component's directory for the full license text.
