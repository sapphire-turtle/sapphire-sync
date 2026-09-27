//! sapphire-sync: sync-only reference app for sapphire-framework.
//!
//! Single binary, Syncthing-style: invoked without a subcommand it runs the
//! embedded server; every other subcommand is a one-shot CLI invocation that
//! talks to the server over the local socket.

use clap::Parser;

use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::server::FrameworkCommand;
use sapphire_sync_core::framework::workspace::AppKind;

// The parse types live in their own module so the integration tests can reach
// them: a `[[bin]]`-only package has no library target, so an integration test
// compiles this module into its own crate through a `#[path]` include of the
// same file.
#[path = "cli.rs"]
pub mod cli;

mod server;
mod sync;

fn main() {
    let cli = cli::Cli::parse();

    // First statement after parsing: what this process is decides where its
    // state lives (and the secrets migration it runs). A bare invocation and
    // `serve` are the server; every other verb is a one-shot CLI.
    CTX.init(match cli.app {
        None | Some(cli::AppCommand::Framework(FrameworkCommand::Serve)) => AppKind::Server,
        _ => AppKind::Cli,
    });

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("sapphire-sync: {err}");
            std::process::exit(1);
        }
    };
    if let Err(err) = runtime.block_on(run(cli)) {
        // A plain `Result` main would print `Error: {err:?}` — the app's own
        // format is the one the plan pins: the program name, the message
        // (Display, not the Debug dump), exit 1.
        eprintln!("sapphire-sync: {err}");
        std::process::exit(1);
    }
}

async fn run(cli: cli::Cli) -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    match cli.app {
        // The app's own verbs come first. The workspace argument is the same
        // global `--workspace-dir` the framework's verbs resolve on their side.
        Some(cli::AppCommand::Sync(command)) => {
            let code = sync::dispatch(command, cli.workspace.workspace_dir).await?;
            std::process::exit(code);
        }
        // Every framework verb. One-shots dispatch against a bridge-less
        // server — `status` on a host without a running server must say the
        // framework's "no … server is running", not a bridge error — and the
        // exit code is the process's.
        Some(cli::AppCommand::Framework(command)) => {
            let code = command.dispatch(server::build_oneshot(), version).await?;
            std::process::exit(code);
        }
        // The parse default is `Serve`: a bare
        // invocation *is* the framework's server verb, which is the only place
        // the bridge is opened.
        None => {
            let server = server::build_serve().await?;
            let code = FrameworkCommand::default()
                .dispatch(server, version)
                .await?;
            std::process::exit(code);
        }
    }
}
