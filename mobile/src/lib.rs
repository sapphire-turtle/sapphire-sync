//! sapphire-sync on a phone: the desktop's screens, with the bridge and the server hosted
//! in this process (the framework's `gui::mobile`).
//!
//! Android loads this crate as `libsapphire_sync_mobile.so` from a GameActivity and calls
//! [`android_main`]. iOS will link the same crate as a static library and call an entry of
//! the same shape; everything below the entry is shared.

use sapphire_framework::gui::app::AppSetup;
use sapphire_framework::gui::mobile::{BuildServer, MobileOptions};
use sapphire_framework::workspace::AppKind;
use sapphire_sync_core::CTX;
use sapphire_sync_ui::App;

/// The app's server, once the framework has put the directories in place: this process is
/// the server, so the context is initialised as one.
pub fn build_server() -> BuildServer {
    Box::new(|| {
        CTX.init(AppKind::Server);
        sapphire_sync_core::serve::build_serve().map_err(Into::into)
    })
}

/// The app's identity and window, the same on every platform.
pub fn options() -> MobileOptions {
    MobileOptions::new(
        sapphire_sync_ui::app(env!("CARGO_PKG_VERSION")),
        "Sapphire Sync",
    )
}

/// The app's screens.
pub fn create(setup: AppSetup<'_>) -> Box<dyn sapphire_framework::gui::eframe::App> {
    Box::new(App::new(setup))
}

/// GameActivity's entry: the activity's `AndroidApp`, run until Android destroys it.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: sapphire_framework::gui::mobile::AndroidApp) {
    if let Err(err) =
        sapphire_framework::gui::mobile::run_android(app, options(), build_server(), create)
    {
        // Logcat is installed by then: the last word on why the window could not run.
        tracing::error!("sapphire-sync could not run: {err}");
    }
}
