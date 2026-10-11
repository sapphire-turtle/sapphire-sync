//! `CTX` resolves its directories from the `SAPPHIRE_SYNC_*_DIR` overrides.
//!
//! Rust 2024 made `std::env::set_var`/`remove_var` unsafe because the
//! environment is process-global while the test harness runs tests on
//! parallel threads. Every env mutation in this binary goes through one
//! static `Mutex` held for the whole test, with an [`EnvGuard`] restoring the
//! previous values on drop — the same pattern as the framework's
//! `sapphire-framework-server` tests. Holding the lock for the whole test is
//! what makes every test binary in this repository single-`CTX`-safe.

use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

use sapphire_sync_core::{CTX, framework};

/// The env vars `CTX.init` reads (`SAPPHIRE_SYNC_{CACHE,DATA,CONFIG}_DIR`).
const DIR_VARS: [&str; 3] = [
    "SAPPHIRE_SYNC_CACHE_DIR",
    "SAPPHIRE_SYNC_DATA_DIR",
    "SAPPHIRE_SYNC_CONFIG_DIR",
];

/// One lock for the whole test binary: env vars are process-global and the
/// test harness runs tests on parallel threads.
static ENV: Mutex<()> = Mutex::new(());

/// Lock the process environment for this test.
fn lock() -> MutexGuard<'static, ()> {
    // A poisoned lock only means some other test panicked while holding it;
    // the environment is not invariant-critical here.
    ENV.lock().unwrap_or_else(|e| e.into_inner())
}

/// Points the context's directories at the test's scratch tree, and restores
/// the previous values when dropped — including while unwinding from a panic.
struct EnvGuard {
    previous: [Option<OsString>; 3],
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// Set all three dir vars to `cache`/`data`/`config` under `root`.
    fn point_at(root: &std::path::Path) -> Self {
        let lock_guard = lock();
        let previous = DIR_VARS.map(std::env::var_os);
        for (name, dir) in DIR_VARS
            .iter()
            .zip(["cache", "data", "config"].map(|cat| root.join(cat)))
        {
            // SAFETY: `lock_guard` serialises every env mutation in this test
            // binary, and it is held until `drop` has restored the old values.
            unsafe { std::env::set_var(name, &dir) };
        }
        EnvGuard {
            previous,
            _lock: lock_guard,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: `self._lock` still serialises the environment; it is dropped
        // only after this method returns.
        for (name, previous) in DIR_VARS.iter().zip(self.previous.iter_mut()) {
            match previous.take() {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

#[test]
fn init_resolves_the_three_directories_from_the_env_overrides() {
    let tmp = tempfile::tempdir().unwrap();
    let _env = EnvGuard::point_at(tmp.path());

    CTX.init(framework::workspace::AppKind::Cli);

    // Each override replaces the *platform root* only: the resolved path is
    // `<override>/<app_name>`.
    for (actual, category) in [
        (CTX.cache_dir(), "cache"),
        (CTX.data_dir(), "data"),
        (CTX.config_dir(), "config"),
    ] {
        let expected = tmp.path().join(category);
        assert_eq!(
            actual,
            expected.join(CTX.app_name),
            "{} directory should be <override>/{}",
            category,
            CTX.app_name
        );
    }
}

#[test]
fn the_facade_rides_along_as_a_re_export() {
    // The CLI depends on the core crate only; the framework facade must be
    // reachable through the core crate's `framework` re-export.
    let ctx = framework::workspace::AppContext::new("sapphire-sync");
    assert_eq!(ctx.app_name, "sapphire-sync");
}
