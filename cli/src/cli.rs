//! CLI-shell types: the command line of the one binary this app ships.
//!
//! A `[[bin]]`-only package has no library target for an integration test to
//! link, so the clap types live in this module and `main.rs` pulls it in with
//! `#[path]`; each test includes it the same way. The module is compiled
//! unconditionally (not behind `cfg(test)`): the tests must be able to include
//! it, and a module of plain clap-type definitions adds nothing to the shipped
//! binary to hide.

use clap::{Parser, Subcommand};

use sapphire_sync_core::framework::workspace::WorkspaceArgs;

// The framework's command enum, re-exported so the tests' `use cli::…` reads
// the same surface `main.rs` sees through `sapphire_sync_core`.
pub use sapphire_sync_core::framework::server::FrameworkCommand;

/// The command line of the one binary this app ships.
///
/// A bare invocation prints the help; the service manager starts the binary
/// with `serve`.
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
    /// The framework's global `--workspace-dir`. Still parsed, but no verb reads
    /// it: every one-shot verb acts on the server's current workspace
    /// (sapphire-framework#227 tracks removing it).
    #[command(flatten)]
    pub workspace: WorkspaceArgs,
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
