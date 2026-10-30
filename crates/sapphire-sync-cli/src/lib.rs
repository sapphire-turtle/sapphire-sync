//! `sapphire-sync`: the app's command line, Syncthing-style.
//!
//! As `serve`, it is the embedded server (`sapphire-sync-server`); every other
//! subcommand is a one-shot invocation that talks to that server over the
//! local socket — the app's own `sync` verb and the framework's (`status`,
//! `service`, `workspace`, `workgroup`, …).
//!
//! The modules are a library so the integration tests can link them, and so
//! both editions run the same commands ([`run`]): the headless one
//! (`sapphire-sync-headless`) and the desktop app, which runs them when given
//! a subcommand. Both put `sapphire-sync` on PATH.

pub mod cli;
pub mod sync;

use std::path::{Path, PathBuf};

use sapphire_sync_core::CTX;
use sapphire_sync_core::framework::server::FrameworkCommand;
use sapphire_sync_core::framework::workspace::AppKind;
use sapphire_sync_server as server;

pub use sapphire_sync_server::Edition;

use crate::cli::AppCommand;

/// Run one command as `edition`, exiting the process with its code.
///
/// An error is returned for the caller to print with [`exit_with`], in the
/// app's own `sapphire-sync: {err}` shape.
pub fn run(command: AppCommand, edition: Edition) -> anyhow::Result<()> {
    // First statement after parsing: what this process is decides where its
    // state lives (and the secrets migration it runs). `serve` is the server;
    // every other verb is a one-shot CLI.
    CTX.init(match command {
        AppCommand::Framework(FrameworkCommand::Serve) => AppKind::Server,
        _ => AppKind::Cli,
    });

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(dispatch(command, edition))
}

/// Print `err` the app's way — the program name, the message (Display, not the
/// Debug dump) — and exit 1. A plain `Result` main would print
/// `Error: {err:?}` instead.
pub fn exit_with(err: anyhow::Error) -> ! {
    eprintln!("sapphire-sync: {err}");
    std::process::exit(1);
}

async fn dispatch(command: AppCommand, edition: Edition) -> anyhow::Result<()> {
    match command {
        // The app's own verbs come first.
        AppCommand::Sync(command) => {
            let code = sync::dispatch(command).await?;
            std::process::exit(code);
        }
        // `serve` is the server the service manager starts (`service_spec` is
        // `["serve"]`), so it must get the sync-mounted server, not the
        // one-shot one. This is the only place the bridge is opened. The
        // framework builds it again on every in-place restart.
        AppCommand::Framework(FrameworkCommand::Serve) => {
            let exe = std::env::current_exe()?;
            let code = FrameworkCommand::Serve
                .dispatch(
                    &CTX,
                    (),
                    |settings| server::build_serve(settings, &exe, edition),
                    server::VERSION,
                )
                .await?;
            std::process::exit(code);
        }
        // Warned first: the exit code is the report's.
        AppCommand::Framework(FrameworkCommand::Status) => {
            warn_if_shadowed();
            oneshot(FrameworkCommand::Status, edition).await
        }
        AppCommand::Framework(command) => oneshot(command, edition).await,
    }
}

/// Every framework verb but `serve`. One-shots dispatch against a bridge-less
/// server — `status` on a host without a running server must say the
/// framework's "no … server is running", not a bridge error — and the exit
/// code is the process's. The app has no settings of its own (`()`).
async fn oneshot(command: FrameworkCommand, edition: Edition) -> anyhow::Result<()> {
    let code = command
        .dispatch(
            &CTX,
            (),
            |settings| server::build_oneshot(settings, edition),
            server::VERSION,
        )
        .await?;
    std::process::exit(code);
}

/// The command both editions put on PATH.
const COMMAND: &str = "sapphire-sync";

/// Warn when the `sapphire-sync` on PATH is another executable than this one:
/// with both the headless edition and the desktop app installed, the one a
/// shell or an MCP client runs may not be the one the service was registered
/// with.
fn warn_if_shadowed() {
    let Ok(this) = std::env::current_exe().and_then(std::fs::canonicalize) else {
        return;
    };
    let Some(on_path) = std::env::var_os("PATH").and_then(|path| find_on_path(&path)) else {
        return;
    };
    if std::fs::canonicalize(&on_path).is_ok_and(|found| found != this) {
        eprintln!(
            "warning: `{COMMAND}` on PATH is {}, not this executable ({}); \
             keep only one of the headless edition and the desktop app installed",
            on_path.display(),
            this.display(),
        );
    }
}

/// The first `sapphire-sync` executable in `path`, a `PATH` value.
fn find_on_path(path: &std::ffi::OsStr) -> Option<PathBuf> {
    let file = format!("{COMMAND}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(path)
        .map(|dir| dir.join(&file))
        .find(|candidate| Path::is_file(candidate))
}
