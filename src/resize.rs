use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};

use crate::output_image;

pub fn load_image(path: &Path) -> Result<DynamicImage> {
    let img = image::open(path).with_context(|| format!("Failed to open image: {}", path.display()))?;
    let (w, h) = img.dimensions();
    if w != h {
        eprintln!("Warning: image is {}x{}, not square. Icons may appear stretched.", w, h);
    }
    if w < 1024 || h < 1024 {
        eprintln!("Warning: image is {}x{}, smaller than 1024x1024. Upscaled icons may look blurry.", w, h);
    }
    Ok(img)
}

/// Resize image to `size x size` pixels.
/// If `fill_white_bg` is true, composites onto a white background first (Apple platform behavior).
/// Returns PNG-encoded bytes.
pub fn resize_icon(img: &DynamicImage, size: u32, fill_white_bg: bool) -> Result<Vec<u8>> {
    let source = if fill_white_bg {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut white = RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255]));
        // Composite source onto white background
        for y in 0..h {
            for x in 0..w {
                let px = rgba.get_pixel(x, y);
                let alpha = px[3] as f32 / 255.0;
                let bg = white.get_pixel(x, y);
                let blended = image::Rgba([
                    blend(px[0], bg[0], alpha),
                    blend(px[1], bg[1], alpha),
                    blend(px[2], bg[2], alpha),
                    255,
                ]);
                white.put_pixel(x, y, blended);
            }
        }
        DynamicImage::ImageRgba8(white)
    } else {
        img.clone()
    };

    let resized = source.resize_exact(size, size, FilterType::Lanczos3);

    let mut buf = Cursor::new(Vec::new());
    if fill_white_bg {
        DynamicImage::ImageRgb8(output_image::flatten_rgba_on_white(&resized.to_rgba8()))
            .write_to(&mut buf, ImageFormat::Png)
    } else {
        resized.write_to(&mut buf, ImageFormat::Png)
    }
    .with_context(|| format!("Failed to encode {}x{} PNG", size, size))?;
    Ok(buf.into_inner())
}

fn blend(fg: u8, bg: u8, alpha: f32) -> u8 {
    ((fg as f32 * alpha) + (bg as f32 * (1.0 - alpha))).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_white_icons_encode_as_rgb_png() {
        let img =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 128])));

        let bytes = resize_icon(&img, 2, true).unwrap();
        let decoder = png::Decoder::new(Cursor::new(bytes));
        let reader = decoder.read_info().unwrap();

        assert_eq!(reader.info().color_type, png::ColorType::Rgb);
    }
}

/// Batch-resize all unique (size, fill_white_bg) combinations.
/// Returns a map from (size, fill_white_bg) -> PNG bytes.
pub fn resize_all(
    img: &DynamicImage,
    sizes: &[(u32, bool)],
) -> Result<HashMap<(u32, bool), Vec<u8>>> {
    let mut map = HashMap::new();
    for &(size, fill_white) in sizes {
        if map.contains_key(&(size, fill_white)) {
            continue;
        }
        let bytes = resize_icon(img, size, fill_white)?;
        map.insert((size, fill_white), bytes);
    }
    Ok(map)
}
