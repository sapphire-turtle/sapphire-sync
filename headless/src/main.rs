//! `sapphire-sync`, the headless edition: the command line of
//! [`sapphire_sync_cli`] and nothing else. The desktop app's binary runs the
//! same commands.

use clap::Parser as _;
use sapphire_sync_cli::Edition;
use sapphire_sync_cli::cli::Cli;

fn main() {
    if let Err(err) = sapphire_sync_cli::run(Cli::parse().app, Edition::Headless) {
        sapphire_sync_cli::exit_with(err);
    }
}
