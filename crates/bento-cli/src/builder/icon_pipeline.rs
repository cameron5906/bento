//! Converts app icons from common source formats (PNG, JPG, SVG) into
//! platform-appropriate formats for the installer and desktop shell.
//!
//! - Windows: .ico (embedded in NSIS installer + taskbar)
//! - macOS: Tauri converts PNG to .icns automatically
//! - Linux: PNG at standard sizes for desktop entries
//!
//! The source icon should ideally be 1024x1024 PNG for best quality.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::output;

/// Generate all platform icon variants from a source image.
/// Returns the path to the generated .ico file (for Windows NSIS).
pub async fn generate_icons(
    source: &Path,
    assets_dir: &Path,
    target: &str,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(assets_dir)?;

    // Always copy the source as icon.png (used by Tauri for macOS/Linux)
    let png_dest = assets_dir.join("icon.png");
    std::fs::copy(source, &png_dest)?;
    output::success("Copied app icon (PNG)");

    // Generate .ico for Windows targets
    if target.starts_with("windows") {
        let ico_path = assets_dir.join("icon.ico");
        match generate_ico(source, &ico_path).await {
            Ok(()) => output::success("Generated app icon (ICO)"),
            Err(e) => {
                output::info(&format!("ICO generation skipped: {}", e));
                // Create a minimal ICO from the PNG as fallback
                if let Err(_) = create_simple_ico(source, &ico_path) {
                    output::info("Using PNG as icon fallback");
                }
            }
        }
    }

    // Generate standard sizes for Linux desktop entries
    if target.starts_with("linux") {
        for size in [32, 64, 128, 256] {
            let sized_name = format!("icon-{}x{}.png", size, size);
            let sized_path = assets_dir.join(&sized_name);
            match resize_png(source, &sized_path, size).await {
                Ok(()) => {}
                Err(_) => {
                    // Fallback: just copy the original
                    let _ = std::fs::copy(source, &sized_path);
                }
            }
        }
    }

    Ok(())
}

/// Use ImageMagick's `magick convert` to generate .ico with multiple sizes
async fn generate_ico(source: &Path, output: &Path) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("magick")
        .args([
            "convert",
            &source.to_string_lossy(),
            "-define", "icon:auto-resize=256,128,64,48,32,16",
            &output.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => anyhow::bail!("magick convert failed"),
        Err(_) => anyhow::bail!("ImageMagick not found (install for ICO generation)"),
    }
}

/// Use ImageMagick to resize a PNG
async fn resize_png(source: &Path, output: &Path, size: u32) -> anyhow::Result<()> {
    let size_str = format!("{}x{}", size, size);
    let status = tokio::process::Command::new("magick")
        .args([
            "convert",
            &source.to_string_lossy(),
            "-resize", &size_str,
            &output.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("resize failed"),
    }
}

/// Create a minimal .ico file by wrapping the PNG data in an ICO container.
/// Works without ImageMagick — produces a valid ICO with a single PNG entry.
fn create_simple_ico(source: &Path, output: &Path) -> anyhow::Result<()> {
    let png_data = std::fs::read(source)?;

    let mut ico = Vec::new();

    // ICO header: reserved(2) + type=1(2) + count=1(2)
    ico.extend_from_slice(&[0, 0]); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    ico.extend_from_slice(&1u16.to_le_bytes()); // count: 1 image

    // Directory entry (16 bytes):
    // width, height (0 = 256), color_count, reserved, planes, bpp, size, offset
    ico.push(0); // width: 0 means 256
    ico.push(0); // height: 0 means 256
    ico.push(0); // color count
    ico.push(0); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    ico.extend_from_slice(&(png_data.len() as u32).to_le_bytes()); // image size
    ico.extend_from_slice(&22u32.to_le_bytes()); // offset (6 header + 16 entry)

    // PNG data
    ico.extend_from_slice(&png_data);

    std::fs::write(output, ico)?;
    Ok(())
}
