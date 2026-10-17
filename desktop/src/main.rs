//! sapphire-sync's desktop app: the app's screens in a window.
//!
//! A pure client: it talks to the service-managed `sapphire-sync serve` and
//! `sapphire-bridge` over IPC and never starts either itself. Closing the window does not
//! stop sync.

use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::eframe;
use sapphire_sync_ui::App;

fn main() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(
        AppOptions::new(
            sapphire_sync_ui::app(env!("CARGO_PKG_VERSION")),
            "Sapphire Sync",
        )
        .min_inner_size([640.0, 420.0]),
        |setup: AppSetup<'_>| Box::new(App::new(setup)),
    )
}
