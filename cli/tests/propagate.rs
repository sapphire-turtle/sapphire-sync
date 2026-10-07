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
    // DIAG(issue #3): record when each direction lands, and dump state on timeout.
    let ok = |h: &common::Host, rel: &str, text: &str| {
        std::fs::read_to_string(h.ws.join(rel)).ok().as_deref() == Some(text)
    };
    let start = std::time::Instant::now();
    common::write(&a, "meeting/notes.md", "agreed");
    common::write(&b, "readme.md", "host b wrote this");
    eprintln!("DIAG step4 writes done at {:?}", start.elapsed());
    let (mut a_to_b, mut b_to_a) = (None, None);
    loop {
        if a_to_b.is_none() && ok(&b, "meeting/notes.md", "agreed") {
            a_to_b = Some(start.elapsed());
            eprintln!("DIAG A->B (meeting/notes.md on B) at {:?}", a_to_b.unwrap());
        }
        if b_to_a.is_none() && ok(&a, "readme.md", "host b wrote this") {
            b_to_a = Some(start.elapsed());
            eprintln!("DIAG B->A (readme.md on A) at {:?}", b_to_a.unwrap());
        }
        if a_to_b.is_some() && b_to_a.is_some() {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            for (name, h) in [("A", &a), ("B", &b)] {
                let rt = h.runtime().unwrap();
                let mut files: Vec<String> = walkdir(&h.ws);
                files.sort();
                eprintln!(
                    "DIAG {name}: files={files:?} live={:?} status={:?}",
                    rt.live_session_devices(&h.ws).await,
                    rt.status(&h.ws).await,
                );
            }
            panic!("DIAG never converged: A->B={a_to_b:?} B->A={b_to_a:?}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    common::assert_roots_match(&a, &b);
}

fn walkdir(root: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p.strip_prefix(root).unwrap().display().to_string());
            }
        }
    }
    out
}

// DIAG(issue #3): the cross-write step alone, with the nested path moved around.
async fn diag_cross(label: &str, a_rel: &str, b_rel: &str) {
    common::init_tracing();
    let net = LoopbackNetwork::new();
    let (a, b) = common::synced_pair(&net).await;
    let start = std::time::Instant::now();
    tracing::warn!(a = %a.ws.display(), b = %b.ws.display(), "DIAG WRITE begin");
    common::write(&a, a_rel, "from a");
    tracing::warn!("DIAG WRITE a done");
    common::write(&b, b_rel, "from b");
    tracing::warn!("DIAG WRITE b done");
    let (mut a_to_b, mut b_to_a) = (None, None);
    while a_to_b.is_none() || b_to_a.is_none() {
        if a_to_b.is_none() && b.ws.join(a_rel).exists() {
            a_to_b = Some(start.elapsed());
        }
        if b_to_a.is_none() && a.ws.join(b_rel).exists() {
            b_to_a = Some(start.elapsed());
        }
        assert!(
            start.elapsed() < std::time::Duration::from_secs(10),
            "DIAG {label} never converged: A->B={a_to_b:?} B->A={b_to_a:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    eprintln!("DIAGV {label}: A->B({a_rel})={a_to_b:?} B->A({b_rel})={b_to_a:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn diag_nested_on_a() {
    diag_cross("nested_on_a", "meeting/notes.md", "readme.md").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn diag_nested_on_b() {
    diag_cross("nested_on_b", "readme.md", "meeting/notes.md").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn diag_flat_both() {
    diag_cross("flat_both", "a.md", "b.md").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn diag_nested_existing_dir_on_a() {
    // The directory exists (and is synced) before the timed write.
    common::init_tracing();
    let net = LoopbackNetwork::new();
    let (a, b) = common::synced_pair(&net).await;
    common::write(&a, "meeting/seed.md", "seed");
    common::poll_until(
        || b.ws.join("meeting/seed.md").exists().then_some(()),
        "seed",
    )
    .await;
    let start = std::time::Instant::now();
    common::write(&a, "meeting/notes.md", "from a");
    common::poll_until(
        || b.ws.join("meeting/notes.md").exists().then_some(()),
        "notes",
    )
    .await;
    eprintln!("DIAGV existing_dir_on_a: A->B={:?}", start.elapsed());
}
