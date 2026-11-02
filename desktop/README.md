# sapphire-sync-desktop

The desktop edition of [sapphire-sync](https://github.com/sapphire-turtle/sapphire-sync) — peer-to-peer file sync over [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework). Built with [egui](https://github.com/emilk/egui) / eframe.

Its binary is also the CLI: run bare it opens the window, and with a subcommand (`serve`, `status`, `sync`, `workspace`, …) it does what `sapphire-sync` does. The window is a client of the service-managed `sapphire-sync serve` and `sapphire-bridge`, which it reaches over IPC and never starts itself; closing it does not stop sync.

## Installation

The desktop app is distributed as an installer for Windows, macOS and Linux on the [Releases](https://github.com/sapphire-turtle/sapphire-sync/releases) page (`sapphire-sync-desktop-win-Setup.exe`, `-osx-Setup.pkg`, `.AppImage`), which puts its binary on PATH as `sapphire-sync` and keeps it up to date. It is a superset of the CLI, so install it instead of the headless edition (`sapphire-sync-headless`), not beside it. The bare binaries (`sapphire-sync-desktop-<os>-<arch>`) can still be run directly. The release workflow builds the package with the `velopack` feature (`cargo build -p sapphire-sync-desktop --features velopack`).

## License

GPL-3.0-or-later, with the [App Store marketplace exception](LICENSE.App-Store-Exception). The binary includes the MIT/Apache-2.0 code of the command line and the server, so it is distributed under GPL-3.0-or-later (with the same exception) as a whole.
