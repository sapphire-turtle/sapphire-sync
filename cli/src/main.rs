//! sapphire-sync: sync-only reference app for sapphire-framework.
//!
//! Single binary, Syncthing-style: invoked without a subcommand it runs the
//! embedded server; every other subcommand is a one-shot CLI invocation that
//! talks to the server over the local socket.

use clap::Parser;

#[derive(Parser)]
#[command(name = "sapphire-sync", version, about = "P2P file sync over sapphire-framework")]
struct Cli {
    // Commands land with the first implementation tasks.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Placeholder until the server lands.
    Run,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Command::Run) => println!("sapphire-sync server: not implemented yet"),
    }
    Ok(())
}
