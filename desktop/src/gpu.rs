// SPDX-License-Identifier: MIT OR Apache-2.0

//! Renderer setup: backend selection, logging, and device-loss reporting.
//!
//! The app asks for Vulkan rather than letting wgpu pick. On Windows the DX12
//! device is removed whenever a Remote Desktop session reconnects or the display
//! configuration changes, and egui-wgpu turns that into an unrecoverable panic
//! (see [`device_lost_explanation`]). `WGPU_BACKEND` still overrides the choice,
//! so `WGPU_BACKEND=dx12` restores the previous behaviour.

use eframe::{egui_wgpu, wgpu};

/// Send `tracing` output, and the `log` records that wgpu emits, to stderr.
///
/// Without this every wgpu diagnostic is discarded, which is what made the
/// device-loss panic so hard to read in the first place. `RUST_LOG` selects the
/// level; `RUST_LOG=wgpu_core=debug,wgpu_hal=debug` is the useful setting when
/// something graphics-related misbehaves.
pub fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .init();
}

/// Renderer configuration for [`eframe::NativeOptions`].
pub fn wgpu_options() -> egui_wgpu::WgpuConfiguration {
    let mut options = egui_wgpu::WgpuConfiguration::default();
    if let egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_setup {
        setup.instance_descriptor.backends = backends();
    }
    options
}

/// Vulkan only, unless `WGPU_BACKEND` says otherwise.
fn backends() -> wgpu::Backends {
    wgpu::Backends::from_env().unwrap_or(wgpu::Backends::VULKAN)
}

/// Report which adapter was chosen and arm the device-loss callback.
///
/// The startup line is how you confirm the Vulkan backend actually took effect.
pub fn on_render_state(render_state: Option<&egui_wgpu::RenderState>) {
    let Some(render_state) = render_state else {
        tracing::warn!("no wgpu render state; skipping GPU diagnostics");
        return;
    };

    let info = render_state.adapter.get_info();
    tracing::info!(
        backend = ?info.backend,
        adapter = %info.name,
        device_type = ?info.device_type,
        driver = %info.driver_info,
        "wgpu renderer ready"
    );

    // Fires before egui-wgpu notices, so this is the only place the real reason
    // is available.
    render_state
        .device
        .set_device_lost_callback(|reason, message| {
            tracing::error!(?reason, %message, "GPU device lost");
        });
}

/// Install a panic hook that translates egui-wgpu's staging-buffer panic.
///
/// The hook only adds an explanation; the previous hook still runs, so the
/// original message and any backtrace are preserved.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(explanation) = panic_message(info).and_then(device_lost_explanation) {
            tracing::error!("{explanation}");
        }
        previous(info);
    }));
}

fn panic_message<'a>(info: &'a std::panic::PanicHookInfo<'_>) -> Option<&'a str> {
    let payload = info.payload();
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}

const DEVICE_LOST_EXPLANATION: &str = concat!(
    "The GPU device was lost. egui-wgpu reports this as a staging buffer failure, ",
    "but the buffer size is not the problem: wgpu swallows a lost device and returns ",
    "None, and egui-wgpu panics on that None. On Windows this usually means a Remote ",
    "Desktop reconnect or a display change removed the graphics device. Restart the app.",
);

/// Recognise egui-wgpu's staging-buffer panic and say what it actually means.
///
/// Both the index-data and vertex-data branches of `Renderer::update_buffers`
/// share the same opening, so one substring covers them. Returns `None` for any
/// other panic message.
pub fn device_lost_explanation(panic_msg: &str) -> Option<&'static str> {
    panic_msg
        .contains("Failed to create staging buffer for ")
        .then_some(DEVICE_LOST_EXPLANATION)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real text from egui-wgpu 0.34.3 `renderer.rs`, index-buffer branch.
    const INDEX_PANIC: &str = "Failed to create staging buffer for index data. \
        Index count: 16113. Required index buffer size: 64452. \
        Actual size 99168 and capacity: 99168 (bytes)";

    /// The vertex-buffer branch of the same function panics with parallel wording.
    const VERTEX_PANIC: &str = "Failed to create staging buffer for vertex data. \
        Vertex count: 4028. Required vertex buffer size: 128896. \
        Actual size 198336 and capacity: 198336 (bytes)";

    #[test]
    fn explains_the_index_buffer_panic() {
        assert!(device_lost_explanation(INDEX_PANIC).is_some());
    }

    #[test]
    fn explains_the_vertex_buffer_panic() {
        assert!(device_lost_explanation(VERTEX_PANIC).is_some());
    }

    #[test]
    fn explanation_names_the_real_cause() {
        let msg = device_lost_explanation(INDEX_PANIC).unwrap();
        assert!(
            msg.contains("device"),
            "should say the device was lost: {msg}"
        );
        assert!(
            msg.contains("Remote Desktop"),
            "should name the usual Windows trigger: {msg}"
        );
    }

    #[test]
    fn ignores_unrelated_panics() {
        assert_eq!(
            device_lost_explanation("index out of bounds: len is 3"),
            None
        );
        assert_eq!(device_lost_explanation(""), None);
    }

    #[test]
    fn defaults_to_vulkan_without_an_env_override() {
        // `backends()` reads WGPU_BACKEND, which is not set in the test harness.
        assert_eq!(backends(), wgpu::Backends::VULKAN);
    }
}
