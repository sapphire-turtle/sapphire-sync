//! Scenario ① — pair and propagate.
//!
//! Two complete hosts on one loopback network (the harness in `e2e/common/mod.rs`,
//! included by path — the package is `[[bin]]`-only, so only top-level `tests/*.rs`
//! files are discovered by cargo and a nested `tests/e2e/` is not). The scenario:
//! enable sync on both hosts, then make one edit at a time and watch it cross the whole
//! stack — client → app server → bridge → peer bridge → peer app server → files — with
//! a poll per edit:
//!
//! 1. a file written on host A appears under host B's root;
//! 2. an edit made on host B converges on host A;
//! 3. a deletion on host A is a deletion on host B;
//! 4. finally the two roots hold exactly the same tree, file by file.
//!
//! Every wait is a deadline-poll on a condition; nothing sleeps blindly.

#![allow(missing_docs)]

#[path = "e2e/common/mod.rs"]
mod common;

use sapphire_sync_core::framework::bridge::LoopbackNetwork;

/// The one scenario of this binary: pair two hosts and propagate an edit each way.
#[tokio::test(flavor = "multi_thread")]
async fn pair_and_propagate() {
    common::init_tracing();
    let net = LoopbackNetwork::new();
    let (a, b) = common::synced_pair(&net).await;

    // 1. A write on A appears under B's root.
    common::write(&a, "note.md", "written on host a");
    let arrived = common::poll_until(
        || std::fs::read_to_string(b.ws.join("note.md")).ok(),
        "note.md written on host a never arrived on host b",
    )
    .await;
    assert_eq!(arrived, "written on host a");

    // 2. An edit on B converges on A.
    common::write(&b, "note.md", "edited on host b");
    common::poll_until(
        || {
            std::fs::read_to_string(a.ws.join("note.md"))
                .ok()
                .filter(|text| text == "edited on host b")
        },
        "the edit made on host b never converged on host a",
    )
    .await;

    // 3. A deletion on A is a deletion on B.
    std::fs::remove_file(a.ws.join("note.md")).unwrap();
    common::poll_until(
        || (!a.ws.join("note.md").exists() && !b.ws.join("note.md").exists()).then_some(()),
        "the deletion made on host a never arrived on host b",
    )
    .await;

    // 4. The same tree on both sides, file by file — after one more write from each
    //    side, so the final compare is of live content, not of an emptied workspace.
    common::write(&a, "meeting/notes.md", "agreed");
    common::write(&b, "readme.md", "host b wrote this");
    common::poll_until(
        || {
            let ok = |h: &common::Host, rel: &str, text: &str| {
                std::fs::read_to_string(h.ws.join(rel)).ok().as_deref() == Some(text)
            };
            (ok(&a, "readme.md", "host b wrote this") && ok(&b, "meeting/notes.md", "agreed"))
                .then_some(())
        },
        "the final cross-writes never converged",
    )
    .await;
    common::assert_roots_match(&a, &b);
}
