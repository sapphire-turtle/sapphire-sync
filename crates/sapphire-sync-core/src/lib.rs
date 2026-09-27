//! Core library for sapphire-sync: the sync-only reference app for
//! sapphire-framework.
//!
//! This crate holds the workspace-level [`CTX`] static and the `framework`
//! facade re-export; the sync logic lands in later tasks.
//!
//! It builds on the `sapphire-framework` facade with the `server` / `sync`
//! features only — no search stack (`retrieve`).

#![warn(missing_docs)]

use sapphire_framework::workspace::AppContext;

/// Application-wide context for sapphire-sync, shared across this app's
/// binaries.
///
/// Declare-and-init-once: the binary calls [`CTX::init`] once at startup,
/// before opening any workspace. Tests point
/// `SAPPHIRE_SYNC_{CACHE,DATA,CONFIG}_DIR` at scratch trees instead, one
/// `Mutex`-guarded env at a time.
pub static CTX: AppContext = AppContext::new("sapphire-sync");

/// The sapphire-framework facade, re-exported so the CLI (which depends on
/// this crate only) reaches the framework through it.
pub use sapphire_framework as framework;
