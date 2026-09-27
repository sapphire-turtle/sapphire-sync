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
/// The framework's verbs ride in flat as one variant of this enum
/// ([`AppCommand::Framework`]), beside the app's own [`AppCommand::Sync`]
/// (plan Deviations 3 and 6). One subcommand enum, every verb at the top
/// level, is the shape the framework's own `command.rs` tests pin: clap
/// rejects a `#[command(flatten)]` *struct field* whose type is
/// `#[derive(Subcommand)]` beside a `#[command(subcommand)]` field — two
/// subcommand enums cannot share one level, and a flattened field must
/// implement `clap::Args`, which the `Subcommand` derive does not provide.
/// A variant carrying the same enum works (the framework's tests use exactly
/// that).
///
/// A bare invocation parses to `app: None`; the framework's half is then the
/// enum's `#[default]` value, [`FrameworkCommand::Serve`], so "run the
/// server" and `serve` are the same thing (command-system decision 1).
#[derive(Parser)]
#[command(
    name = "sapphire-sync",
    version,
    about = "P2P file sync over sapphire-framework"
)]
pub struct Cli {
    /// The app's own verbs (Deviation 6: just `sync`), beside the framework's.
    #[command(subcommand)]
    pub app: Option<AppCommand>,
    /// The global `--workspace-dir` (issue #128), honoured by the app's own
    /// `sync` verb; the framework's workspace verbs resolve their root the
    /// same way on their side.
    #[command(flatten)]
    pub workspace: WorkspaceArgs,
}

/// The command line's top level: the app's own verbs and the framework's, flat.
#[derive(Debug, Subcommand)]
pub enum AppCommand {
    /// Workspace sync controls (Deviation 6: `sync disable` is the one verb
    /// the framework's `workspace` group lacks).
    Sync(SyncCommand),
    /// The framework's verbs, flattened in (spec decision 2).
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
    /// Stop syncing the workspace this invocation resolves to.
    ///
    /// Files and the sync-id stay; re-enabling via `workspace init --sync`
    /// rejoins the same workspace.
    Disable,
}
