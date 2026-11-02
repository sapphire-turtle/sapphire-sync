//! The console shim the Windows package puts on `PATH` as `sapphire-sync.exe` (#33):
//! `sapphire-sync-desktop.exe` is built for the GUI subsystem, so a command typed in a
//! terminal would print nothing. This runs it with the same arguments and streams, waits,
//! and exits with its code (corundum-framework-package's `console::forward`).

#[cfg(windows)]
fn main() -> ! {
    sapphire_framework::gui::package::runtime::console::forward("sapphire-sync-desktop.exe")
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("sapphire-sync-console is the Windows package's command; run sapphire-sync-desktop");
    std::process::ExitCode::FAILURE
}
