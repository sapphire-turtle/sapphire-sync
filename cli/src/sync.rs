//! The app's `sync` verb (Task 3 lands the real dispatch).
//!
//! This module exists so `main`'s match is complete now; the placeholder
//! keeps `sync disable` compiling until the real dispatcher arrives, and its
//! one line is the same the framework's `workgroup create` prints for a verb
//! whose work is elsewhere.

/// Run the parsed `sync` verb.
///
/// Placeholder: the sync dispatcher is Task 3's (`sync::dispatch` in the
/// plan), which talks to the running server over IPC.
pub fn dispatch() -> anyhow::Result<()> {
    eprintln!("sync disable: not implemented yet (task 3)");
    std::process::exit(1);
}
