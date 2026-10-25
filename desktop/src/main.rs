//! sapphire-sync's desktop app: the framework's shell, with its Settings.
//!
//! A pure client: it talks to the service-managed `sapphire-sync serve` and
//! `sapphire-bridge` over IPC and never starts either itself. Closing the window does not
//! stop sync. The Files screen is a placeholder for the synced folder's browser (the
//! mobile app needs one); until then Settings, at the rail's foot, is what the app does.

use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{Destination, Settings, Shell, eframe, egui, i18n, tr};

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-sync",
    version: env!("CARGO_PKG_VERSION"),
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
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui, |ui, screen| match screen {
            Screen::Files => files_ui(ui),
        });
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

fn main() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(
        AppOptions::new(APP, "Sapphire Sync").min_inner_size([640.0, 420.0]),
        |setup: AppSetup<'_>| {
            i18n::register(&setup.cc.egui_ctx, RESOURCES);
            let mut shell = Shell::new(
                vec![Destination::new(
                    Screen::Files,
                    egui::include_image!("../assets/icons/folder.svg"),
                    "sync-files",
                )],
                Settings::new(setup.framework),
            );
            // Settings is all the app does until Files browses the folder.
            shell.open_settings();
            Box::new(App { shell })
        },
    )
}
