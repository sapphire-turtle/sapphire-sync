//! The whole command surface parses, and a bare invocation is `serve`.
//!
//! Modelled on the framework's `command::tests` (the probes that pinned the
//! same shapes for `FrameworkCommand` itself). These tests reach the binary's
//! parse types through a `#[path]` include of the `cli.rs` module `main.rs`
//! also pulls in — a `[[bin]]`-only package has no library target, so the
//! integration test compiles the module into its own crate.

// The module is normally reached as `sapphire_sync::cli`; here it is compiled
// into this test crate directly.
#[path = "../src/cli.rs"]
mod cli;

use clap::Parser;

use cli::{AppCommand, Cli, FrameworkCommand, SyncSubcommand};

/// Parse `args` (after the binary name) and return the parsed `Cli`.
fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("sapphire-sync").chain(args.iter().copied()))
        .unwrap_or_else(|err| panic!("{args:?} must parse: {err}"))
}

/// A bare invocation parses to `app: None`; the framework's half is then the
/// enum's `#[default]`, `Serve` — the same thing the service unit runs.
#[test]
fn a_bare_invocation_is_serve() {
    let cli = parse(&[]);
    assert!(cli.app.is_none(), "bare invocation has no app verb");
    assert_eq!(cli.workspace.workspace_dir, None);
    assert!(matches!(
        FrameworkCommand::default(),
        FrameworkCommand::Serve
    ));
}

#[test]
fn serve_parses_as_the_framework_verb() {
    let cli = parse(&["serve"]);
    match cli.app {
        Some(AppCommand::Framework(FrameworkCommand::Serve)) => {}
        other => panic!("`serve` must parse as the framework's, got {other:?}"),
    }
}

#[test]
fn status_parses() {
    let cli = parse(&["status"]);
    match cli.app {
        Some(AppCommand::Framework(FrameworkCommand::Status)) => {}
        other => panic!("`status` must parse as the framework's, got {other:?}"),
    }
}

#[test]
fn the_service_subcommands_parse() {
    for args in [
        vec!["service", "install", "--user"],
        vec!["service", "uninstall"],
        vec!["service", "status"],
    ] {
        let cli = parse(&args);
        match cli.app {
            Some(AppCommand::Framework(FrameworkCommand::Service(_))) => {}
            other => panic!("{args:?} must parse as the framework's service, got {other:?}"),
        }
    }
}

#[test]
fn the_workspace_subcommands_parse() {
    for args in [
        vec!["workspace", "init"],
        vec!["workspace", "init", "notes"],
        vec!["workspace", "init", "--sync"],
        vec!["workspace", "list"],
        vec!["workspace", "map", "notes", "/srv/notes"],
    ] {
        let cli = parse(&args);
        match cli.app {
            Some(AppCommand::Framework(FrameworkCommand::Workspace(_))) => {}
            other => panic!("{args:?} must parse as the framework's workspace, got {other:?}"),
        }
    }
}

#[test]
fn workgroup_and_device_lists_parse() {
    for args in [vec!["workgroup", "list"], vec!["device", "list"]] {
        let cli = parse(&args);
        match cli.app {
            Some(AppCommand::Framework(FrameworkCommand::Workgroup(_)))
            | Some(AppCommand::Framework(FrameworkCommand::Device(_))) => {}
            other => panic!("{args:?} must parse as the framework's, got {other:?}"),
        }
    }
}

#[test]
fn the_apps_own_sync_verb_parses() {
    let cli = parse(&["sync", "disable"]);
    match cli.app {
        Some(AppCommand::Sync(command)) => {
            assert!(matches!(command.command, SyncSubcommand::Disable));
        }
        other => panic!("`sync disable` must parse as the app's, got {other:?}"),
    }
}

#[test]
fn the_global_workspace_dir_parses_everywhere() {
    // Before the verb, after it, and after a deeper subcommand: `global = true`.
    for args in [
        vec!["--workspace-dir", "/tmp/x", "status"],
        vec!["status", "--workspace-dir", "/tmp/x"],
        vec!["workspace", "list", "--workspace-dir", "/tmp/x"],
        vec!["sync", "disable", "--workspace-dir", "/tmp/x"],
    ] {
        let cli = parse(&args);
        assert_eq!(
            cli.workspace.workspace_dir.as_deref(),
            Some(std::path::Path::new("/tmp/x")),
            "{args:?}"
        );
    }
}

#[test]
fn the_apps_sync_and_the_frameworks_groups_coexist() {
    // The parse-collision check: one level, both families' verbs.
    let Err(help) = Cli::try_parse_from(["sapphire-sync", "--help"]) else {
        panic!("--help must not parse as success")
    };
    let text = help.render().to_string();
    for verb in [
        "sync",
        "workspace",
        "workgroup",
        "device",
        "status",
        "serve",
    ] {
        assert!(text.contains(verb), "--help must list {verb}: {text}");
    }
}
