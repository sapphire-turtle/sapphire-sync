//! The whole command surface parses, and a bare invocation prints the help.
//!
//! Modelled on the framework's `command::tests` (the probes that pinned the
//! same shapes for `FrameworkCommand` itself).

use clap::Parser;

use sapphire_sync_cli::cli::{AppCommand, Cli, FrameworkCommand, SyncSubcommand};

/// Parse `args` (after the binary name) and return the parsed `Cli`.
fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("sapphire-sync").chain(args.iter().copied()))
        .unwrap_or_else(|err| panic!("{args:?} must parse: {err}"))
}

/// A bare invocation prints the help, as the framework recommends; the
/// service unit runs `serve`.
#[test]
fn a_bare_invocation_prints_the_help() {
    let err = Cli::try_parse_from(["sapphire-sync"])
        .err()
        .expect("a bare invocation does not parse");
    assert_eq!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
}

#[test]
fn serve_parses_as_the_framework_verb() {
    let cli = parse(&["serve"]);
    match cli.app {
        AppCommand::Framework(FrameworkCommand::Serve) => {}
        other => panic!("`serve` must parse as the framework's, got {other:?}"),
    }
}

#[test]
fn status_parses() {
    let cli = parse(&["status"]);
    match cli.app {
        AppCommand::Framework(FrameworkCommand::Status) => {}
        other => panic!("`status` must parse as the framework's, got {other:?}"),
    }
}

#[test]
fn the_service_subcommands_parse() {
    for args in [
        vec!["service", "install"],
        vec!["service", "uninstall"],
        vec!["service", "status"],
    ] {
        let cli = parse(&args);
        match cli.app {
            AppCommand::Framework(FrameworkCommand::Service(_)) => {}
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
        vec!["workspace", "select", "notes"],
        vec!["workspace", "show"],
        vec![
            "workspace",
            "init",
            "--sync",
            "--remote-workspace",
            "notes",
            "/srv/notes",
        ],
    ] {
        let cli = parse(&args);
        match cli.app {
            AppCommand::Framework(FrameworkCommand::Workspace(_)) => {}
            other => panic!("{args:?} must parse as the framework's workspace, got {other:?}"),
        }
    }
}

#[test]
fn workgroup_and_device_lists_parse() {
    for args in [vec!["workgroup", "list"], vec!["device", "list"]] {
        let cli = parse(&args);
        match cli.app {
            AppCommand::Framework(FrameworkCommand::Workgroup(_))
            | AppCommand::Framework(FrameworkCommand::Device(_)) => {}
            other => panic!("{args:?} must parse as the framework's, got {other:?}"),
        }
    }
}

#[test]
fn the_apps_own_sync_verb_parses() {
    let cli = parse(&["sync", "disable"]);
    match cli.app {
        AppCommand::Sync(command) => {
            assert!(matches!(command.command, SyncSubcommand::Disable));
        }
        other => panic!("`sync disable` must parse as the app's, got {other:?}"),
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
