//! sapphire-sync's desktop app: the framework's `SyncPanel` in a window.
//!
//! A pure client: it talks to the service-managed `sapphire-sync serve` and
//! `sapphire-bridge` over IPC and never starts either itself. Closing the window does not
//! stop sync.

mod gpu;

use std::sync::Arc;

use sapphire_framework::gui::SyncPanel;
use sapphire_framework::gui::client::{AppIdentity, ClientConfig, FrameworkClient};

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-sync",
    version: env!("CARGO_PKG_VERSION"),
};

struct App {
    panel: SyncPanel,
    // Keeps the client's background task alive for the window's lifetime.
    _runtime: tokio::runtime::Runtime,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        self.panel.ui(ui);
    }
}

fn main() -> eframe::Result<()> {
    gpu::init_logging();
    gpu::install_panic_hook();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime");
    let config = ClientConfig::new(APP).expect("the runtime directory and this executable's path");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("Sapphire Sync"),
        wgpu_options: gpu::wgpu_options(),
        ..Default::default()
    };
    eframe::run_native(
        "Sapphire Sync",
        options,
        Box::new(move |cc| {
            gpu::on_render_state(cc.wgpu_render_state.as_ref());
            sapphire_framework::gui::fonts::install_system_cjk_fallback(&cc.egui_ctx);
            let ctx = cc.egui_ctx.clone();
            let client = FrameworkClient::spawn(
                runtime.handle(),
                config,
                Arc::new(move || ctx.request_repaint()),
            );
            Ok(Box::new(App {
                panel: SyncPanel::new(client),
                _runtime: runtime,
            }))
        }),
    )
}
