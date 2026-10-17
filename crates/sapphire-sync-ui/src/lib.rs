//! sapphire-sync's screens: the framework's shell, with the synced folder's files and
//! Settings — the same on the desktop and on a phone.
//!
//! The desktop window is a pure client of the service-managed `sapphire-sync serve` and
//! `sapphire-bridge`; the phone app hosts both in its own process. Either way the screens
//! only talk to them through the framework's client, so they do not know which.

use sapphire_framework::gui::app::AppSetup;
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::views::FileBrowser;
use sapphire_framework::gui::{Destination, Settings, Shell, eframe, egui};

/// The app, as the framework's client names it, at the `version` of the binary showing
/// these screens (the desktop app and the phone app are versioned on their own).
pub const fn app(version: &'static str) -> AppIdentity {
    AppIdentity {
        app_name: "sapphire-sync",
        version,
    }
}

/// The app's own screens, on the framework's rail.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Files,
}

/// The app's window content.
pub struct App {
    shell: Shell<Screen>,
    files: FileBrowser,
}

impl App {
    /// The app, built from what the framework's start-up hands over.
    pub fn new(setup: AppSetup<'_>) -> App {
        let files = FileBrowser::new(setup.runtime.clone(), setup.repaint.clone());
        let shell = Shell::new(
            vec![Destination::new(
                Screen::Files,
                egui::include_image!("../assets/icons/folder.svg"),
                "Files",
            )],
            Settings::new(setup.framework),
        );
        App { shell, files }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The workspace the server serves, while its folder is there to read.
        let root = self
            .shell
            .settings()
            .client()
            .snapshot()
            .server
            .up()
            .and_then(|server| server.current.as_ref())
            .filter(|current| current.reachable)
            .map(|current| current.root.clone());
        let files = &mut self.files;
        self.shell.ui(ui, |ui, screen| match screen {
            Screen::Files => files.ui(ui, root.as_deref()),
        });
    }
}
