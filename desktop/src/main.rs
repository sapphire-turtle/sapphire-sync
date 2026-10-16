//! sapphire-sync's desktop app: the framework's Settings in a window.
//!
//! A pure client: it talks to the service-managed `sapphire-sync serve` and
//! `sapphire-bridge` over IPC and never starts either itself. Closing the window does not
//! stop sync.

use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{Settings, eframe, egui};

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-sync",
    version: env!("CARGO_PKG_VERSION"),
};

struct App {
    panel: Settings,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.panel.ui(ui);
    }
}

fn main() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(
        AppOptions::new(APP, "Sapphire Sync").min_inner_size([640.0, 420.0]),
        |setup: AppSetup<'_>| {
            Box::new(App {
                panel: Settings::new(setup.framework),
            })
        },
    )
}
