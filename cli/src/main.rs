//! sapphire-sync: sync-only reference app for sapphire-framework.
//!
//! Single binary, Syncthing-style: invoked without a subcommand it runs the
//! embedded server; every other subcommand is a one-shot CLI invocation that
//! talks to the server over the local socket.

use clap::Parser;

use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::server::FrameworkCommand;
use sapphire_sync_core::framework::workspace::AppKind;

// The parse types live in their own module so the integration tests can link
// them (`sapphire_sync::cli`): a `[[bin]]`-only package has no library target,
// so an integration test's `use sapphire_sync::cli::…` links this same module
// through a `#[path]` include of its own.
#[path = "cli.rs"]
pub mod cli;

mod server;
mod sync;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    // First statement after parsing: what this process is decides where its
    // state lives (and the secrets migration it runs). A bare invocation and
    // `serve` are the server; every other verb is a one-shot CLI.
    CTX.init(match cli.app {
        None | Some(cli::AppCommand::Framework(FrameworkCommand::Serve)) => AppKind::Server,
        _ => AppKind::Cli,
    });

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run(cli))
}

async fn run(cli: cli::Cli) -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    match cli.app {
        // The app's own verbs come first; Task 3 lands the real `sync`
        // dispatch, so the arm is a placeholder until then.
        Some(cli::AppCommand::Sync(_)) => sync::dispatch(),
        // Bare invocation and every framework verb.
        Some(cli::AppCommand::Framework(command)) => {
            let server = server::build().await?;
            let code = command.dispatch(server, version).await?;
            std::process::exit(code);
        }
        // The parse default is `Serve` (command-system decision 1): a bare
        // invocation *is* the framework's server verb.
        None => {
            let server = server::build().await?;
            FrameworkCommand::default()
                .dispatch(server, version)
                .await?;
            Ok(())
        }
    }
}
