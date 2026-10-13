//! What a client says to the sapphire-sync server beyond the framework's methods, and
//! the version of it the server speaks.
//!
//! Kept apart from the app so that its version moves only when this API does: a
//! sapphire-sync release that leaves this crate alone does not demand a service
//! restart. It depends on nothing, so a client that only talks to the server links
//! neither the framework nor the IPC transport through it.
//!
//! The server serves the framework's common methods (`sapphire-framework-server-api`,
//! versioned on its own and checked by the handshake's `api`) and, so far, no methods
//! of its own: `sync disable` is the framework's `sync.disable`. This crate is where
//! the app's own methods and their types go when it gains some.

#![warn(missing_docs)]

/// The version of the sapphire-sync server's own API, which the server reports in the
/// handshake (`ServerInfo.app_api`) and a client of this app names (`ClientInfo.app_api`).
///
/// It is this crate's major version, parsed at compile time, so the two cannot drift: a
/// breaking change to a method, a parameter or a result here is a major release of this
/// crate, and that release is the new API version.
///
/// 1: the first release; the framework's methods only.
pub const API_VERSION: u32 = parse_major(env!("CARGO_PKG_VERSION_MAJOR"));

/// `CARGO_PKG_VERSION_MAJOR` as a number. Cargo guarantees it is decimal digits.
const fn parse_major(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut n = 0u32;
    while i < bytes.len() {
        n = n * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_version_is_the_crate_major() {
        let major: u32 = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
        assert_eq!(API_VERSION, major);
        assert_eq!(API_VERSION, 1);
    }
}
