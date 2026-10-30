//! CLI-shell types: the command line both editions run.

use clap::{Parser, Subcommand};

// The framework's command enum, re-exported so the tests' `use cli::…` reads
// the same surface `run` sees through `sapphire_sync_core`.
pub use sapphire_sync_core::framework::server::FrameworkCommand;

/// The command line of the headless edition's `sapphire-sync` binary.
///
/// A bare invocation prints the help; the service manager starts the binary
/// with `serve`. The desktop app parses the same [`AppCommand`], with a bare
/// invocation opening its window instead.
#[derive(Parser)]
#[command(
    name = "sapphire-sync",
    arg_required_else_help = true,
    version,
    about = "P2P file sync over sapphire-framework"
)]
pub struct Cli {
    /// The app's own verbs, beside the framework's.
    #[command(subcommand)]
    pub app: AppCommand,
}

/// The command line's top level: the app's own verbs and the framework's, flat.
#[derive(Debug, Subcommand)]
pub enum AppCommand {
    /// Workspace sync controls: `sync disable`.
    Sync(SyncCommand),
    /// The framework's verbs, flattened in.
    #[command(flatten)]
    Framework(FrameworkCommand),
}

/// The app's own `sync` verb group: `sync disable`, no more.
#[derive(Debug, clap::Args)]
pub struct SyncCommand {
    /// The verb itself.
    #[command(subcommand)]
    pub command: SyncSubcommand,
}

/// The `sync` verbs.
#[derive(Debug, Subcommand)]
pub enum SyncSubcommand {
    /// Stop syncing the server's workspace (refused from inside another one).
    ///
    /// Files and the sync-id stay; re-enabling via `workspace init --sync`
    /// rejoins the same workspace.
    Disable,
}
