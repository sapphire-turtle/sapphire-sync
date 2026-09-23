//! Core library for sapphire-sync: the sync-only reference app for
//! sapphire-framework.
//!
//! This crate holds the sync logic; the `sapphire-sync` binary (with the
//! embedded server) is the only consumer for now. It builds on the
//! `sapphire-framework` facade with the `server` / `sync` features only —
//! no search stack (`retrieve`).
