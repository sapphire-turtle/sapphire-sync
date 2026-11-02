//! sapphire-sync's desktop app: the framework's shell, with its Settings — and,
//! given a subcommand, the CLI.
//!
//! This one binary is a superset of the CLI's: bare, it opens the window; with
//! `serve`, `status`, `sync`, `workspace`, … it runs the CLI's own code
//! (`sapphire_sync_cli::run`), so the service manager can start it as it starts
//! `sapphire-sync`.
//!
//! The window is a pure client: it talks to the service-managed `sapphire-sync serve` and
//! `sapphire-bridge` over IPC and never starts either itself. Closing the window does not
//! stop sync. The Files screen is a placeholder for the synced folder's browser (the
//! mobile app needs one); until then Settings, at the rail's foot, is what the app does.
//!
//! With the `velopack` feature (the release workflow's build) it is a Velopack package
//! (src/package.rs): it updates itself, and its first start asks to keep the app server
//! running.

// In the package, a GUI binary on Windows: no console window behind the window or the
// service. The command line goes through the console shim (src/bin/console.rs).
#![cfg_attr(all(windows, feature = "velopack"), windows_subsystem = "windows")]

#[cfg(feature = "velopack")]
mod package;

use clap::Parser;
use corundum_framework_platform::launcher::LauncherSpec;
use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{
    Destination, Settings, Shell, ShellApp, Side, eframe, egui, i18n, tr,
};
use sapphire_framework::server::FrameworkCommand;
use sapphire_sync_cli::cli::AppCommand;

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-sync",
    version: env!("CARGO_PKG_VERSION"),
};

/// Sapphire Sync in the OS's app menu, and the window's id and icon. As the package's
/// (`release-binaries.yml`): `org.sapphire-turtle.sapphire-sync` is its macOS bundle id and
/// `Network` its Linux category. Not the packId (`sapphire-sync-desktop`), as corundum's demo.
const LAUNCHER: LauncherSpec = LauncherSpec {
    id: "sapphire-sync",
    name: "Sapphire Sync",
    description: "Peer-to-peer file sync for Sapphire workspaces",
    category: "Network",
    icon_svg: include_bytes!("../assets/app-icon.svg"),
    bundle_prefix: "org.sapphire-turtle",
};

/// The app's own strings, beside the framework's: the rail's label and the Files screen.
const RESOURCES: i18n::Resources = i18n::Resources {
    name: "sapphire-sync-desktop",
    files: &[
        ("en", include_str!("../locales/en/sapphire-sync.ftl")),
        ("ja", include_str!("../locales/ja/sapphire-sync.ftl")),
    ],
};

/// The app's own screens, on the framework's rail.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Files,
}

struct App {
    shell: Shell<Screen>,
    screens: Screens,
}

/// What the shell draws: the screens. The side pane keeps the shell's default (empty)
/// until the Files screen has items to open in it.
struct Screens;

impl ShellApp<Screen> for Screens {
    fn screen_ui(&mut self, ui: &mut egui::Ui, screen: Screen, _side: &mut Side) {
        match screen {
            Screen::Files => files_ui(ui),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.show(ui, &mut self.screens);
    }
}

/// The Files screen, until it browses the synced folder.
fn files_ui(ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            ui.heading(tr!(ui, "sync-files"));
            ui.label(tr!(ui, "sync-files-placeholder"));
        });
    });
}

/// The CLI's command line, with the subcommand optional: none opens the window.
#[derive(Parser)]
#[command(
    name = "sapphire-sync",
    about = "P2P file sync over sapphire-framework: the desktop app, or the CLI's commands",
    version
)]
struct DesktopCli {
    #[command(subcommand)]
    command: Option<AppCommand>,
}

/// Whether this start goes through the package's start: every start but `serve`, the
/// package's service (the start starts the services an update stopped, and that is this
/// process). Velopack's hooks pass arguments clap rejects, so this is decided before clap
/// may exit on them.
#[cfg_attr(not(feature = "velopack"), allow(dead_code))]
fn runs_package_start(parsed: &Result<DesktopCli, clap::Error>) -> bool {
    !matches!(
        parsed,
        Ok(DesktopCli {
            command: Some(AppCommand::Framework(FrameworkCommand::Serve)),
        })
    )
}

fn main() -> eframe::Result<()> {
    let parsed = DesktopCli::try_parse();
    // Velopack's start: a hook runs and exits here. The window's console first, so the
    // hooks log; a command installs its own.
    #[cfg(feature = "velopack")]
    if runs_package_start(&parsed) {
        if !matches!(parsed, Ok(DesktopCli { command: Some(_) })) {
            sapphire_framework::gui::app::install_logging(APP);
        }
        let _ = sapphire_framework::gui::package::runtime::start(&package::PACKAGE);
    }
    match parsed.unwrap_or_else(|e| e.exit()).command {
        // In the package, the package's updater replaces it, not the desktop release file.
        #[cfg(feature = "velopack")]
        Some(AppCommand::Framework(FrameworkCommand::Update(args)))
            if corundum_framework_platform::package::current().is_some() =>
        {
            std::process::exit(package_update(&args))
        }
        Some(command) => match sapphire_sync_cli::run(command, sapphire_sync_cli::Edition::Desktop)
        {
            Ok(()) => Ok(()),
            Err(err) => sapphire_sync_cli::exit_with(err),
        },
        None => window(),
    }
}

/// `update` in the package: `sapphire-bridge` first (unless `--no-bridge`), as the package's
/// own update ends this process, then the package through its updater. The exit code.
#[cfg(feature = "velopack")]
fn package_update(args: &sapphire_framework::server::AppUpdateArgs) -> i32 {
    let bridge = if args.no_bridge {
        0
    } else {
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime
                .block_on(sapphire_framework::server::update_bridge(&args.update))
                .unwrap_or_else(|err| {
                    eprintln!("sapphire-bridge: {err}");
                    1
                }),
            Err(err) => {
                eprintln!("sapphire-sync: {err}");
                1
            }
        }
    };
    let own = match package::APP_PACKAGE.updater() {
        Some(updater) => package::APP_PACKAGE.update(
            &updater,
            args.update.check,
            &mut std::io::stdout(),
            std::time::Duration::from_secs(5 * 60),
        ),
        None => 1,
    };
    own.max(bridge)
}

fn window() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(app_options(), |setup: AppSetup<'_>| {
        i18n::register(&setup.cc.egui_ctx, RESOURCES);
        let settings = Settings::new(setup.framework);
        // About's service row and updates, in the package.
        #[cfg(feature = "velopack")]
        let settings = settings.with_package_about(setup.package_about);
        let mut shell = Shell::new(
            vec![Destination::new(
                Screen::Files,
                egui::include_image!("../assets/icons/folder.svg"),
                "sync-files",
            )],
            settings,
        );
        // Settings is all the app does until Files browses the folder.
        shell.open_settings();
        Box::new(App {
            shell,
            screens: Screens,
        })
    })
}

/// What the window opens with: the package's icon and id, and in the package, the package.
fn app_options() -> AppOptions {
    let options = AppOptions::new(APP, "Sapphire Sync")
        .min_inner_size([640.0, 420.0])
        .launcher(LAUNCHER);
    #[cfg(feature = "velopack")]
    let options = options.package(package::APP_PACKAGE);
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    // Velopack's hooks start the binary with arguments clap does not know; the package's
    // start must see them before clap exits on them.
    #[test]
    fn package_start_runs_before_clap_rejects() {
        let parsed = |args: &[&str]| DesktopCli::try_parse_from(args);
        assert!(runs_package_start(&parsed(&[
            "x",
            "--veloapp-install",
            "1.0.0"
        ])));
        assert!(runs_package_start(&parsed(&["x", "--version"])));
        assert!(runs_package_start(&parsed(&["x"])));
        assert!(runs_package_start(&parsed(&["x", "status"])));
        assert!(!runs_package_start(&parsed(&["x", "serve"])));
    }

    // The window shows the package's icon even outside a package, and goes by the
    // package's bundle id, so the OS groups it with the installer's entry.
    #[test]
    fn the_window_has_sapphire_syncs_icon_and_id() {
        assert_eq!(LAUNCHER.id, "sapphire-sync");
        assert_eq!(LAUNCHER.name, "Sapphire Sync");
        assert_eq!(LAUNCHER.category, "Network");
        assert_eq!(LAUNCHER.bundle_prefix, "org.sapphire-turtle");
        assert_eq!(
            LAUNCHER.icon_svg,
            include_bytes!("../assets/app-icon.svg").as_slice()
        );
        assert!(corundum_framework_platform::icon::render_rgba(LAUNCHER.icon_svg, 64).is_ok());
    }
}
