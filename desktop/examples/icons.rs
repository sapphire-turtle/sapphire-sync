//! Writes Sapphire Sync's icon as `icon.ico`, `icon.icns` and `icon.png` (512 px) into the
//! directory given, for `vpk pack --icon` in the release workflow (`release-binaries.yml`).

fn main() -> std::io::Result<()> {
    use corundum_framework_platform::icon;

    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("usage: icons <dir>"));
    std::fs::create_dir_all(&dir)?;
    let svg = include_bytes!("../assets/app-icon.svg");
    std::fs::write(dir.join("icon.ico"), icon::ico(svg)?)?;
    std::fs::write(dir.join("icon.icns"), icon::icns(svg)?)?;
    std::fs::write(dir.join("icon.png"), icon::render_png(svg, 512)?)?;
    Ok(())
}
