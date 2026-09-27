//! The app's `sync` verb (Task 3 lands the real dispatch).
//!
//! This module exists so `main`'s match is complete now; the placeholder
//! returns an error rather than exiting, so main's error path prints it in the
//! app's own `sapphire-sync: {err}` shape and sets the exit code in one place.

use crate::cli::SyncCommand;

/// Run the parsed `sync` verb.
///
/// Placeholder: the sync dispatcher is Task 3's (`sync::dispatch` in the
/// plan), which talks to the running server over IPC.
pub fn dispatch(command: SyncCommand) -> anyhow::Result<()> {
    match command.command {
        crate::cli::SyncSubcommand::Disable => Err(anyhow::anyhow!(
            "sync disable: not implemented yet (task 3)"
        )),
    }
}
