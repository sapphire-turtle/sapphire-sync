//! Scenario ③ — the missing-root guard.
//!
//! Two complete hosts on one loopback network (the harness in `e2e/common/mod.rs`,
//! included by path — the package is `[[bin]]`-only, so only top-level `tests/*.rs`
//! files are cargo-discovered and a nested `tests/e2e/` is not). The scenario: take a
//! synced workspace's root away from A, and check that sync *stops* rather than
//! replicating the disappearance as a mass deletion.
//!
//! ## What the framework actually does (measured — see `task-5-probe-findings.md`)
//!
//! The plan expected `SyncRuntime::status` to report `paused: Some("RootMissing")` after
//! the root was renamed away. It does not, and the reason is structural:
//!
//! - `SyncRuntime::status` (and `scan`) canonicalize the root *first*, and a root that no
//!   longer exists fails that canonicalize. The workspace is then reported as
//!   **not enabled** (`enabled: false, paused: None`), not as paused.
//! - `PauseReason::RootMissing` is therefore unreachable through `status` by renaming the
//!   root away. It *is* reachable when the root path still resolves but is not a
//!   directory — replace it with a regular file — and the replica-level guard still
//!   exists underneath (the framework's own
//!   `sapphire-framework-sync/tests/replica_guards.rs` asserts `RootMissing` at that
//!   layer, and `MarkerMissing` when only the marker directory goes).
//!
//! This file therefore asserts the two behaviours the app *does* surface:
//!
//! 1. renamed away → the workspace stops being reported as a synced workspace at all
//!    (`enabled: false` and **no** pause reason), which is still a stop in effect: no
//!    scan, no replication, no mass deletion;
//! 2. the same absence expressed as a non-directory root → `paused: Some("RootMissing")`.
//!
//! Both are checks that the peer's files are untouched. The recovery half is asserted
//! through `enabled` coming back and a fresh write from B arriving, because a root that
//! is renamed away leaves `paused` empty (there is no reason string to clear).
//!
//! Every wait is a deadline-poll on a condition; nothing sleeps blindly.

#![allow(missing_docs)]

#[path = "e2e/common/mod.rs"]
mod common;

use sapphire_sync_core::framework::bridge::LoopbackNetwork;

/// A renamed-away root stops the workspace syncing, and the peer is untouched.
///
/// Named for what it asserts: the workspace stops being reported as synced (it is not
/// enabled, and carries no pause reason), rather than pausing with a `RootMissing` reason.
#[tokio::test(flavor = "multi_thread")]
async fn a_vanished_root_stops_reporting_synced_instead_of_deleting_everything() {
    common::init_tracing();
    let net = LoopbackNetwork::new();
    let (a, b) = common::synced_pair(&net).await;

    // A file both hosts hold, so a mass deletion would have something to delete.
    common::write(&a, "notes/a.md", "base");
    common::poll_until(
        || (common::read_text(&b, "notes/a.md").as_deref() == Some("base")).then_some(()),
        "the base file written on host a never arrived on host b",
    )
    .await;
    common::settle(&[&a, &b]).await;

    // The drive is unmounted: the whole root goes, marker directory and all.
    let parked = a.ws.with_file_name("notes-parked");
    std::fs::rename(&a.ws, &parked).unwrap();

    // The root no longer resolves, so the workspace is not reported as a synced one any
    // more — the structural consequence of `status` canonicalizing first (see the module
    // docs). What matters here is the *effect*: replication stops, and the peer keeps its
    // files.
    let status = common::poll_until_async(
        || async {
            let status = a.runtime().unwrap().status(&a.ws).await;
            (!status.enabled).then_some(status)
        },
        "host a never stopped reporting the vanished workspace as synced",
    )
    .await;
    // The measured state, asserted rather than left implicit: this path reports no pause
    // *reason* at all (the canonicalize fails before the guard is reached).
    assert!(
        status.paused.is_none(),
        "a renamed-away root surfaces no pause reason, but got {:?}",
        status.paused
    );
    assert_eq!(
        common::read_text(&b, "notes/a.md").as_deref(),
        Some("base"),
        "the peer lost its file when host a's root vanished"
    );

    // Bring the root back and confirm the workspace re-enables, then that a write from B
    // reaches A again — the recovery half, through `enabled` rather than `paused`.
    std::fs::rename(&parked, &a.ws).unwrap();
    common::poll_until_async(
        || async {
            let status = a.runtime().unwrap().status(&a.ws).await;
            status.enabled.then_some(())
        },
        "host a never re-enabled the workspace after its root came back",
    )
    .await;
    common::settle(&[&a, &b]).await;

    common::write(&b, "notes/recovered.md", "after the root came back");
    common::poll_until(
        || {
            (common::read_text(&a, "notes/recovered.md").as_deref()
                == Some("after the root came back"))
            .then_some(())
        },
        "a file written on host b after recovery never reached host a",
    )
    .await;

    common::assert_roots_match(&a, &b);
}

/// A root that is present but not a directory is reported as `RootMissing`.
///
/// This is the form in which the framework's `PauseReason::RootMissing` is actually
/// reachable through `SyncRuntime::status`: the path resolves for the canonicalize, but
/// the replica's own guard sees a root that is not a directory. `MarkerMissing` is the
/// sibling case, when the root is a directory but its marker is gone.
#[tokio::test(flavor = "multi_thread")]
async fn a_root_that_is_not_a_directory_is_reported_as_root_missing() {
    common::init_tracing();
    let net = LoopbackNetwork::new();
    let (a, b) = common::synced_pair(&net).await;

    common::write(&a, "notes/a.md", "base");
    common::poll_until(
        || (common::read_text(&b, "notes/a.md").as_deref() == Some("base")).then_some(()),
        "the base file written on host a never arrived on host b",
    )
    .await;
    common::settle(&[&a, &b]).await;

    // Park the root and put a regular file where it was: the path still resolves, so
    // `status` reaches the replica's guard, and the guard is the one that says
    // `RootMissing`.
    let parked = a.ws.with_file_name("notes-parked");
    std::fs::rename(&a.ws, &parked).unwrap();
    std::fs::write(&a.ws, "not a directory").unwrap();

    // The guard is evaluated by a scan; the watcher's debounce triggers one, and an
    // explicit `scan` makes the trigger deterministic rather than waiting on the watcher.
    a.runtime().unwrap().scan(&a.ws).await.unwrap();
    let status = a.runtime().unwrap().status(&a.ws).await;
    assert_eq!(
        status.paused.as_deref(),
        Some("RootMissing"),
        "a root that is not a directory must be reported as RootMissing"
    );
    assert_eq!(
        common::read_text(&b, "notes/a.md").as_deref(),
        Some("base"),
        "the peer lost its file when host a's root was replaced"
    );

    // Put the root back and confirm the pause clears.
    std::fs::remove_file(&a.ws).unwrap();
    std::fs::rename(&parked, &a.ws).unwrap();
    a.runtime().unwrap().scan(&a.ws).await.unwrap();
    let status = a.runtime().unwrap().status(&a.ws).await;
    assert_eq!(
        status.paused, None,
        "the pause must clear once the root is a directory again"
    );

    common::assert_roots_match(&a, &b);
}
