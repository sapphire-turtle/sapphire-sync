# sapphire-sync-headless

The headless edition of [sapphire-sync](https://github.com/sapphire-turtle/sapphire-sync) — peer-to-peer
file sync over [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework) — for hosts
without a GUI. Its one binary, `sapphire-sync`, is the app server (`sapphire-sync serve`) and the CLI
that talks to it. The desktop app's binary runs the same commands and is put on PATH under the same
name.

## Installation

### Install script

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- sync
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) sync
```

This installs `sapphire-sync` together with `sapphire-bridge`, which it syncs through, into
`~/.local/bin`.

### From crates.io

```sh
cargo binstall sapphire-sync-headless   # the release file
cargo install sapphire-sync-headless    # or build it
```

Either installs the `sapphire-sync` binary. The desktop app's binary runs the same commands; install
only one of the two.

## Usage

See the [repository's README](https://github.com/sapphire-turtle/sapphire-sync#quick-start).

## License

MIT OR Apache-2.0.
