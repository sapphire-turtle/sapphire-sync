//! Scenario ④ — filtering and the size limit.
//!
//! Two complete hosts on one loopback network (the harness in `e2e/common/mod.rs`,
//! included by path — the package is `[[bin]]`-only, so only top-level `tests/*.rs`
//! files are cargo-discovered and a nested `tests/e2e/` is not). Three claims about what
//! sync refuses to carry, each checked at the files:
//!
//! 1. `.sapphireignore` itself is synced — a peer needs it to filter the same way — so a
//!    `*.tmp` rule written on A is present on B too;
//! 2. a file the rule excludes (`x.tmp`) never reaches B, while its sibling does;
//! 3. a file larger than `DEFAULT_MAX_FILE_SIZE` (64 MiB) is skipped — silently, so the
//!    file-level absence is the real assertion — while its smaller sibling syncs.
//!
//! ## Why the rule is written before enabling, and the write that carries it comes after
//!
//! The filter is read from the root when the workspace is opened, so the rule has to exist
//! *before* sync is enabled — which is why this file builds its hosts by hand instead of
//! using [`common::synced_pair`], whose `enable` runs first. But a file that exists before
//! enabling is not by itself carried: `SyncRuntime::enable` dials a session and does *not*
//! scan, so nothing has recorded the pre-enable file yet (measured —
//! `task-5-probe-findings.md`). The first write *after* enabling is what drives the
//! watcher's scan, and that scan records the whole root, ignore file included. So the
//! scenario writes the rule before enabling and its first ordinary file after, which is
//! also the order a user follows: configure, enable, then work.
//!
//! The ignored and oversize files are deliberately *local to A*: they are exactly what
//! both roots do not agree on, so the final comparison is over the paths sync does carry,
//! named one by one rather than as a whole-root equality.
//!
//! The oversize payload is built by setting the length and writing a small pattern rather
//! than allocating 64 MiB of a byte: the content is never read back, only its byte count
//! matters to the size check, and `set_len` keeps the test's memory flat.

#![allow(missing_docs)]

#[path = "e2e/common/mod.rs"]
mod common;

use sapphire_sync_core::framework::bridge::LoopbackNetwork;
use sapphire_sync_core::framework::sync::DEFAULT_MAX_FILE_SIZE;

/// The ignore rule and the size limit, both at the files on the two roots.
#[tokio::test(flavor = "multi_thread")]
async fn filtering_and_size_limits_hold_at_the_files() {
    common::init_tracing();
    let net = LoopbackNetwork::new();

    let a = common::start_host(&net, common::NODE_A, "host-a").await;
    let b = common::start_host(&net, common::NODE_B, "host-b").await;
    common::introduce(&a, &b);
    common::write(&a, ".sapphireignore", "*.tmp\n");
    common::enable_sync(&a).await;
    common::enable_sync(&b).await;
    common::settle(&[&a, &b]).await;

    // 1 and 2 together: the post-enable write drives the scan that records the rule, and
    // the rule then decides its own sibling's fate. The excluded file stays on A; the
    // sibling and the ignore file both cross.
    common::write(&a, "x.tmp", "excluded");
    common::write(&a, "kept.txt", "carried");
    common::poll_until(
        || (common::read_text(&b, "kept.txt").as_deref() == Some("carried")).then_some(()),
        "the sibling of the excluded file never reached host b",
    )
    .await;
    common::poll_until(
        || (common::read_text(&b, ".sapphireignore").as_deref() == Some("*.tmp\n")).then_some(()),
        "the .sapphireignore written on host a never arrived on host b",
    )
    .await;
    assert_eq!(
        common::read_text(&b, "x.tmp"),
        None,
        "a *.tmp file was synced even though .sapphireignore excludes it"
    );

    // 3. Over the limit: skipped (silently — no message crosses), while the small sibling
    //    still syncs.
    let oversize = a.ws.join("big.bin");
    {
        use std::io::Write as _;
        let mut file = std::fs::File::create(&oversize).unwrap();
        file.write_all(b"oversize").unwrap();
        file.set_len(DEFAULT_MAX_FILE_SIZE + 1).unwrap();
    }
    common::write(&a, "small.bin", "small");
    common::poll_until(
        || (common::read_text(&b, "small.bin").as_deref() == Some("small")).then_some(()),
        "the sibling of the oversize file never reached host b",
    )
    .await;
    assert_eq!(
        common::read_text(&b, "big.bin"),
        None,
        "a file over DEFAULT_MAX_FILE_SIZE was synced"
    );

    // Everything sync does carry agrees on both roots — the ignore file and the two small
    // files — while the ignored and oversize files remain A's alone. Asserted path by
    // path, since a whole-root equality would (correctly) fail on exactly those two.
    for rel in [".sapphireignore", "kept.txt", "small.bin"] {
        assert_eq!(
            common::read_text(&a, rel),
            common::read_text(&b, rel),
            "{rel} differs between the hosts"
        );
    }
    assert!(a.ws.join("x.tmp").exists(), "the excluded file left host a");
    assert!(
        !b.ws.join("x.tmp").exists(),
        "the excluded file reached host b"
    );
    assert!(oversize.exists(), "the oversize file left host a");
    assert!(
        !b.ws.join("big.bin").exists(),
        "the oversize file reached host b"
    );
}
