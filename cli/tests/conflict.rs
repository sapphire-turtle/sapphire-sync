//! Scenario ② — a conflict copy for a concurrent edit.
//!
//! Two complete hosts on one loopback network (the harness in `e2e/common/mod.rs`,
//! included by path — the package is `[[bin]]`-only, so only top-level `tests/*.rs`
//! files are cargo-discovered and a nested `tests/e2e/` is not). The scenario:
//! sync `notes/a.md` with identical content on both hosts, then write *different*
//! content on A and on B. Nothing names a winner: whichever side's HLC sorts higher
//! keeps the path, the loser's version lands beside it as a conflict copy, and both
//! roots must end up agreeing on that.
//!
//! Three ordering rules make the scenario deterministic, and all were measured with a
//! probe before this file was written (see `task-5-probe-findings.md`):
//!
//! 1. **Settle before writing.** The pair is [`common::settle`]d only *after* the base
//!    file has arrived, so both live sessions are open before the concurrent writes.
//! 2. **Wait for each side to have recorded its own edit** before the exchange below, so
//!    the local scans (the watcher's debounce) have run and the replicas each hold their
//!    own version.
//! 3. **Re-exchange until the roots agree.** The framework's live path is best-effort —
//!    an update can be dropped while a session is opening, and the documented recovery is
//!    the *next* exchange ("the peer's vector does not cover it, so the next dial or push
//!    carries it again"). Closing the sessions and letting the dialer rebuild them is
//!    that exchange, exactly what a reconnect does in the field. One exchange is
//!    sometimes enough and sometimes not, so the loop is bounded by a deadline rather
//!    than by a round count.
//!
//! Every wait is a deadline-poll on a condition; nothing sleeps blindly.

#![allow(missing_docs)]

#[path = "e2e/common/mod.rs"]
mod common;

use std::time::{Duration, Instant};

use sapphire_sync_core::framework::bridge::LoopbackNetwork;

/// The one scenario of this binary: concurrent edits keep both versions on both hosts.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_edits_keep_both_versions() {
    common::init_tracing();
    let net = LoopbackNetwork::new();
    let (a, b) = common::synced_pair(&net).await;

    // The base state: one file, the same bytes on both hosts. Then wait for the pair to
    // have open live sessions, so the two writes below start from a settled pair.
    common::write(&a, "notes/a.md", "base");
    common::poll_until(
        || (common::read_text(&b, "notes/a.md").as_deref() == Some("base")).then_some(()),
        "the base file written on host a never arrived on host b",
    )
    .await;
    common::settle(&[&a, &b]).await;

    // Two edits at the same path, one from each side, neither aware of the other. The
    // order is irrelevant: both are committed.
    common::write(&a, "notes/a.md", "from a");
    common::write(&b, "notes/a.md", "from b");

    // Each side's own edit has to be *recorded* before the exchange, or an exchange could
    // run before the local scan committed it and carry nothing.
    common::poll_until(
        || (common::read_text(&a, "notes/a.md").as_deref() == Some("from a")).then_some(()),
        "host a never recorded its own edit",
    )
    .await;
    common::poll_until(
        || (common::read_text(&b, "notes/a.md").as_deref() == Some("from b")).then_some(()),
        "host b never recorded its own edit",
    )
    .await;

    // Re-exchange until both roots hold the same two names with the two versions. A
    // converged pair needs no further exchange; the loop only pays for the cases where an
    // update was dropped.
    let names = converge(&a, &b).await;

    // The winner keeps the path; the other name is the conflict copy, named per the
    // framework's `merge::conflict_path`: `<stem>.conflict-<grain-id>-<counter>.<ext>`.
    // The loser's replica id is not reachable through the runtime's public API, so the
    // name's *shape* is asserted, not a computed id — the framework's own
    // `sapphire-framework-sync/tests/replica_conflict.rs` asserts the same way.
    let others: Vec<&String> = names.iter().filter(|name| *name != "a.md").collect();
    assert_eq!(
        others.len(),
        1,
        "expected exactly one conflict copy in {names:?}"
    );
    let copy = others[0];
    assert!(
        copy.starts_with("a.conflict-") && copy.ends_with(".md"),
        "the conflict copy is not named per merge::conflict_path: {copy}"
    );

    // The two versions are exactly the two writes — the winner's at the shared path, the
    // loser's in the copy. Whichever side won, the pair is the same.
    let winner = common::read_text(&a, "notes/a.md").unwrap();
    let loser = common::read_text(&a, &format!("notes/{copy}")).unwrap();
    let mut both = [winner.as_str(), loser.as_str()];
    both.sort();
    assert_eq!(
        both,
        ["from a", "from b"],
        "the surviving versions are not the two concurrent writes"
    );

    // And the two roots agree byte for byte, conflict copy included.
    common::assert_roots_match(&a, &b);
}

/// Force exchanges until both roots hold the winner and one conflict copy of both writes,
/// and return the agreed set of names under `notes/`.
///
/// The converged condition is the *whole* final state, not just the names: a root can
/// list the winner and the copy a moment before the copy's bytes are the loser's, so
/// polling on names alone would read a mid-write tree. The loop gives up at a deadline
/// with both roots printed, which is the state a real divergence would leave.
async fn converge(a: &common::Host, b: &common::Host) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let settled = |left: &common::Host, right: &common::Host| -> Option<Vec<String>> {
            let on_left = common::files_under(left, "notes");
            let on_right = common::files_under(right, "notes");
            if on_left.len() != 2 || on_left != on_right {
                return None;
            }
            let mut contents = Vec::new();
            for name in &on_left {
                contents.push(common::read_text(left, &format!("notes/{name}"))?);
            }
            for name in &on_right {
                contents.push(common::read_text(right, &format!("notes/{name}"))?);
            }
            contents.sort();
            (contents == ["from a", "from a", "from b", "from b"]).then_some(on_left)
        };

        if let Some(names) = settled(a, b) {
            return names;
        }
        assert!(
            Instant::now() < deadline,
            "the concurrent edits never converged: a={:?} b={:?}",
            common::files_under(a, "notes"),
            common::files_under(b, "notes")
        );

        // One exchange: drop the live sessions and wait for the dialer to rebuild both
        // sides before deciding again.
        a.runtime().unwrap().drop_connections().await;
        b.runtime().unwrap().drop_connections().await;
        common::poll_until_async(
            || async {
                let left_open = !a
                    .runtime()
                    .unwrap()
                    .live_session_devices(&a.ws)
                    .await
                    .is_empty();
                let right_open = !b
                    .runtime()
                    .unwrap()
                    .live_session_devices(&b.ws)
                    .await
                    .is_empty();
                (left_open && right_open).then_some(())
            },
            "the exchanged sessions never came back",
        )
        .await;
    }
}
