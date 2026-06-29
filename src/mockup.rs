use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, RgbaImage};

use crate::device::{self, DeviceConfig, OrientationConfig};

const MARKER_KEY: &str = "launch";
const MARKER_VALUE: &str = "mockup";

/// Embedded macOS desktop wallpaper for MacBook mockups.
const MAC_DESKTOP_BG: &[u8] = include_bytes!("../resources/mac-desktop-bg.jpg");

/// Resize screenshot to fit device display resolution with letterboxing.
fn fit_to_resolution(img: &DynamicImage, width: u32, height: u32, auto_rotate: bool, fill_ratio: f64, bg: Option<&RgbaImage>) -> RgbaImage {
    let (iw, ih) = img.dimensions();
    let img_ratio = iw as f64 / ih as f64;
    let dev_ratio = width as f64 / height as f64;

    // Auto-rotate if screenshot orientation doesn't match device (phones only)
    let img = if auto_rotate && (img_ratio > 1.0) != (dev_ratio > 1.0) {
        img.rotate90()
    } else {
        img.clone()
    };
    let (iw, ih) = img.dimensions();

    // Scale to fit, applying fill_ratio to leave margins
    let max_w = (width as f64 * fill_ratio) as u32;
    let max_h = (height as f64 * fill_ratio) as u32;
    let scale = (max_w as f64 / iw as f64).min(max_h as f64 / ih as f64);
    let new_w = (iw as f64 * scale).round() as u32;
    let new_h = (ih as f64 * scale).round() as u32;
    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

    // Background: use provided image or solid black
    let mut canvas = if let Some(bg_img) = bg {
        let bg_resized = image::imageops::resize(bg_img, width, height, image::imageops::FilterType::Lanczos3);
        RgbaImage::from_raw(width, height, bg_resized.into_raw()).unwrap()
    } else {
        RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 255]))
    };

    let offset_x = (width - new_w) / 2;
    let offset_y = (height - new_h) / 2;
    image::imageops::overlay(
        &mut canvas,
        &resized.to_rgba8(),
        offset_x as i64,
        offset_y as i64,
    );
    canvas
}

/// Load an image from embedded bytes.
fn load_from_bytes(bytes: &[u8]) -> Result<RgbaImage> {
    let img = image::load_from_memory(bytes).context("Failed to decode embedded image")?;
    Ok(img.to_rgba8())
}

/// Compose the final mockup image.
fn compose(
    screenshot: &DynamicImage,
    device: &DeviceConfig,
    orientation: &OrientationConfig,
    background: Option<&RgbaImage>,
) -> Result<RgbaImage> {
    let (dw, dh) = device.display_resolution;

    // For landscape, swap the display resolution
    let (dw, dh) = if orientation.name == "landscape" {
        (dh, dw)
    } else {
        (dw, dh)
    };

    // Auto-rotate only for phone orientations (portrait/landscape), not for "front" (laptops)
    let auto_rotate = orientation.name == "portrait" || orientation.name == "landscape";
    // For laptops, scale down to leave margins around the app window
    let fill_ratio = if orientation.name == "front" { 0.85 } else { 1.0 };

    // Use embedded desktop wallpaper for "front" (laptop) when no custom background
    let default_bg;
    let bg = if orientation.name == "front" && background.is_none() {
        default_bg = image::load_from_memory(MAC_DESKTOP_BG)
            .context("Failed to decode embedded wallpaper")?
            .to_rgba8();
        Some(&default_bg)
    } else {
        background
    };

    // 1. Resize screenshot to display resolution
    let fitted = fit_to_resolution(screenshot, dw, dh, auto_rotate, fill_ratio, bg);

    // 2. Load template and mask from embedded bytes
    let template = load_from_bytes(orientation.template)?;
    let mask = load_from_bytes(orientation.mask)?;

    let (tw, th) = (template.width(), template.height());

    // 3. Calculate screen area from coords (TL corner)
    let coords = &orientation.screen_coord;
    let min_x = coords.iter().map(|c| c.0).min().unwrap();
    let min_y = coords.iter().map(|c| c.1).min().unwrap();

    // 4. Compose: for each pixel, decide what to show
    //    - template opaque → show template (device frame)
    //    - template transparent + mask opaque → show screenshot (screen area)
    //    - both transparent → nothing (outside device)
    let mut result = RgbaImage::from_pixel(tw, th, image::Rgba([0, 0, 0, 0]));

    for y in 0..th {
        for x in 0..tw {
            let tpl_px = template.get_pixel(x, y);
            let mask_px = mask.get_pixel(x, y);
            let tpl_a = tpl_px[3] as f32 / 255.0;

            if tpl_a > 0.99 {
                // Fully opaque frame pixel — show template directly
                result.put_pixel(x, y, *tpl_px);
            } else if mask_px[3] > 0 {
                // Screen area — show screenshot, blend template on top if semi-transparent
                let sx = x as i64 - min_x as i64;
                let sy = y as i64 - min_y as i64;
                let scr_px = if sx >= 0
                    && sy >= 0
                    && (sx as u32) < fitted.width()
                    && (sy as u32) < fitted.height()
                {
                    *fitted.get_pixel(sx as u32, sy as u32)
                } else {
                    image::Rgba([0, 0, 0, 255]) // black letterbox
                };

                if tpl_a < 0.01 {
                    // Fully transparent template — pure screenshot
                    result.put_pixel(x, y, scr_px);
                } else {
                    // Semi-transparent template edge — blend frame over screenshot
                    let r = (tpl_px[0] as f32 * tpl_a + scr_px[0] as f32 * (1.0 - tpl_a)) as u8;
                    let g = (tpl_px[1] as f32 * tpl_a + scr_px[1] as f32 * (1.0 - tpl_a)) as u8;
                    let b = (tpl_px[2] as f32 * tpl_a + scr_px[2] as f32 * (1.0 - tpl_a)) as u8;
                    result.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                }
            }
            // else: outside device — stays transparent
        }
    }

    Ok(result)
}

/// Check if a PNG file was already processed by launch.
pub fn is_already_processed(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let decoder = png::Decoder::new(BufReader::new(file));
    let Ok(reader) = decoder.read_info() else {
        return false;
    };
    reader
        .info()
        .uncompressed_latin1_text
        .iter()
        .any(|t| t.keyword == MARKER_KEY && t.text == MARKER_VALUE)
}

/// Save an RGBA image as PNG with the launch marker.
fn save_with_marker(img: &RgbaImage, output: &Path) -> Result<()> {
    let file = File::create(output)
        .with_context(|| format!("Failed to create {}", output.display()))?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, img.width(), img.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.add_text_chunk(MARKER_KEY.to_string(), MARKER_VALUE.to_string())?;
    let mut writer = encoder.write_header()?;
    writer.write_image_data(img.as_raw())?;
    writer.finish()?;
    Ok(())
}

/// Run the mockup generation.
pub fn run(
    input: &Path,
    output: &Path,
    device_id: &str,
    orientation_name: &str,
) -> Result<()> {
    run_with_bg(input, output, device_id, orientation_name, None)
}

/// Run the mockup generation with an optional desktop background.
pub fn run_with_bg(
    input: &Path,
    output: &Path,
    device_id: &str,
    orientation_name: &str,
    background: Option<&Path>,
) -> Result<()> {
    let device =
        device::find_device(device_id).with_context(|| format!("Unknown device: {}", device_id))?;

    let orientation = device.find_orientation(orientation_name).with_context(|| {
        let available: Vec<_> = device.orientations.iter().map(|o| o.name).collect();
        format!(
            "Unknown orientation '{}' for {}. Available: {}",
            orientation_name,
            device.name,
            available.join(", ")
        )
    })?;

    let screenshot = image::open(input)
        .with_context(|| format!("Failed to open screenshot: {}", input.display()))?;

    eprintln!(
        "Generating {} {} mockup...",
        device.name, orientation_name
    );

    let bg_img = if let Some(bg_path) = background {
        Some(image::open(bg_path)
            .with_context(|| format!("Failed to open background: {}", bg_path.display()))?
            .to_rgba8())
    } else {
        None
    };

    let result = compose(&screenshot, device, orientation, bg_img.as_ref())?;
    save_with_marker(&result, output)?;

    eprintln!("Saved mockup to {}", output.display());
    Ok(())
}

/// Print available devices.
pub fn list_devices() {
    eprintln!("Available devices:");
    for device in device::all_devices() {
        let orientations: Vec<_> = device.orientations.iter().map(|o| o.name).collect();
        eprintln!(
            "  {} ({} {}) — {}",
            device.id, device.name, device.color,
            orientations.join(", ")
        );
    }
}
