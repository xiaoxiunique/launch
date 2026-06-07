use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, Rgb, RgbImage, RgbaImage};

const WHITE: [u8; 3] = [255, 255, 255];

pub fn flatten_rgba_on_white(img: &RgbaImage) -> RgbImage {
    let mut out = RgbImage::new(img.width(), img.height());
    for (x, y, px) in img.enumerate_pixels() {
        let alpha = px[3] as f32 / 255.0;
        out.put_pixel(
            x,
            y,
            Rgb([
                blend(px[0], WHITE[0], alpha),
                blend(px[1], WHITE[1], alpha),
                blend(px[2], WHITE[2], alpha),
            ]),
        );
    }
    out
}

pub fn save_opaque_png(img: &RgbaImage, output: &Path) -> Result<()> {
    DynamicImage::ImageRgb8(flatten_rgba_on_white(img))
        .save(output)
        .with_context(|| format!("Failed to save image to {}", output.display()))
}

fn blend(fg: u8, bg: u8, alpha: f32) -> u8 {
    ((fg as f32 * alpha) + (bg as f32 * (1.0 - alpha))).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_rgba_on_white_removes_alpha() {
        let img = RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 128]));

        let out = flatten_rgba_on_white(&img);

        assert_eq!(out.get_pixel(0, 0).0, [127, 127, 127]);
    }
}
