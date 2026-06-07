use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::RgbaImage;

use crate::mockup;
use crate::output_image;
use crate::screenshot;

/// Generate a collage from multiple screenshots arranged in an auto grid.
pub fn run(
    images: &[PathBuf],
    output: &Path,
    device_id: &str,
    orientation: &str,
    padding_override: Option<u32>,
    no_frame: bool,
) -> Result<()> {
    if images.is_empty() {
        anyhow::bail!("No images to collage");
    }

    // 1. Process each image: apply mockup frame unless --no-frame
    let mut mockup_images: Vec<RgbaImage> = Vec::with_capacity(images.len());
    let tmp_dir = std::env::temp_dir();

    for (i, path) in images.iter().enumerate() {
        let img = if no_frame || mockup::is_already_processed(path) {
            if mockup::is_already_processed(path) {
                eprintln!("[{}/{}] {} (already framed)", i + 1, images.len(), path.display());
            } else {
                eprintln!("[{}/{}] {} (no frame)", i + 1, images.len(), path.display());
            }
            image::open(path)
                .with_context(|| format!("Failed to open {}", path.display()))?
                .to_rgba8()
        } else {
            eprintln!("[{}/{}] {} -> mockup", i + 1, images.len(), path.display());
            let tmp = tmp_dir.join(format!("launch-collage-{}.png", i));
            mockup::run(path, &tmp, device_id, orientation)?;
            let img = image::open(&tmp)
                .with_context(|| format!("Failed to open mockup result for {}", path.display()))?
                .to_rgba8();
            let _ = std::fs::remove_file(&tmp);
            img
        };
        mockup_images.push(img);
    }

    // 2. Calculate grid layout: cols = ceil(sqrt(n))
    let n = mockup_images.len();
    let cols = (n as f64).sqrt().ceil() as u32;
    let rows = ((n as f64) / cols as f64).ceil() as u32;

    // 3. Use first image dimensions as cell size reference, scale all to match
    let cell_w = mockup_images[0].width();
    let cell_h = mockup_images[0].height();

    // Auto-scale padding: more cells → more breathing room
    // Base 3% of cell width, grows with grid density
    let padding = padding_override.unwrap_or_else(|| {
        let base = (cell_w as f32 * 0.03) as u32;
        let scale = 1.0 + (cols.max(rows) - 1) as f32 * 0.4;
        (base as f32 * scale) as u32
    });

    let resized: Vec<RgbaImage> = mockup_images
        .iter()
        .map(|img| {
            if img.width() == cell_w && img.height() == cell_h {
                img.clone()
            } else {
                image::imageops::resize(
                    img,
                    cell_w,
                    cell_h,
                    image::imageops::FilterType::Lanczos3,
                )
            }
        })
        .collect();

    // 4. Calculate canvas dimensions
    let canvas_w = cols * cell_w + (cols + 1) * padding;
    let canvas_h = rows * cell_h + (rows + 1) * padding;

    // 5. Create gradient background canvas
    let mut canvas = screenshot::create_gradient_canvas(canvas_w, canvas_h);

    // 6. Place each image in the grid
    for (i, img) in resized.iter().enumerate() {
        let col = i as u32 % cols;
        let row = i as u32 / cols;
        let x = padding + col * (cell_w + padding);
        let y = padding + row * (cell_h + padding);
        image::imageops::overlay(&mut canvas, img, x as i64, y as i64);
    }

    // 7. Save
    output_image::save_opaque_png(&canvas, output)
        .with_context(|| format!("Failed to save collage to {}", output.display()))?;

    eprintln!(
        "Collage: {} images in {}x{} grid -> {}",
        n, cols, rows, output.display()
    );

    Ok(())
}
