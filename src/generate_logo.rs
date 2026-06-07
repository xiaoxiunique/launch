use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use anyhow::{Context, Result};
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_line_segment_mut, draw_polygon_mut, draw_text_mut,
};
use imageproc::point::Point;

use crate::output_image;

const OUTPUT_SIZE: u32 = 1024;
const SCALE: u32 = 3;
const CANVAS_SIZE: u32 = OUTPUT_SIZE * SCALE;
const EMBEDDED_FONT: &[u8] = include_bytes!("../resources/YouSheBiaoTiHei.ttf");

const PALETTES: [([u8; 3], [u8; 3], [u8; 3]); 8] = [
    ([28, 86, 246], [88, 214, 255], [255, 255, 255]),
    ([11, 132, 96], [143, 224, 120], [255, 252, 235]),
    ([230, 70, 58], [255, 187, 83], [32, 18, 12]),
    ([106, 67, 255], [255, 109, 207], [255, 255, 255]),
    ([18, 24, 38], [68, 214, 182], [248, 250, 252]),
    ([244, 124, 45], [255, 220, 97], [32, 24, 20]),
    ([37, 99, 235], [167, 139, 250], [255, 255, 255]),
    ([16, 185, 129], [14, 165, 233], [4, 24, 33]),
];

#[derive(Debug, Clone)]
pub struct LogoOptions {
    pub output: PathBuf,
    pub text: Option<String>,
    pub seed: Option<u64>,
}

pub fn run(options: LogoOptions) -> Result<()> {
    if let Some(parent) = options.output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }

    let seed = options.seed.unwrap_or_else(random_seed);
    let image = render_logo(seed, options.text.as_deref())?;
    output_image::save_opaque_png(&image, &options.output)
        .with_context(|| format!("Failed to save logo: {}", options.output.display()))?;
    eprintln!("Logo seed: {}", seed);
    Ok(())
}

fn render_logo(seed: u64, text: Option<&str>) -> Result<RgbaImage> {
    let mut rng = SmallRng::new(seed);
    let palette = PALETTES[rng.range_usize(PALETTES.len())];
    let style = rng.range_usize(5);

    let mut canvas = gradient_canvas(palette.0, palette.1);
    add_soft_shapes(&mut canvas, &mut rng, palette);
    draw_symbol(&mut canvas, &mut rng, style, palette);

    if let Some(text) = text.and_then(clean_logo_text) {
        draw_logo_text(&mut canvas, &text, palette.2)?;
    }

    Ok(image::imageops::resize(
        &canvas,
        OUTPUT_SIZE,
        OUTPUT_SIZE,
        FilterType::Lanczos3,
    ))
}

fn random_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    nanos ^ std::process::id() as u64
}

fn gradient_canvas(top: [u8; 3], bottom: [u8; 3]) -> RgbaImage {
    let mut img = RgbaImage::new(CANVAS_SIZE, CANVAS_SIZE);
    for y in 0..CANVAS_SIZE {
        let t = y as f32 / (CANVAS_SIZE - 1) as f32;
        let wave = ((t * std::f32::consts::PI).sin() * 0.08).max(0.0);
        for x in 0..CANVAS_SIZE {
            let diagonal = x as f32 / (CANVAS_SIZE - 1) as f32;
            let mix = (t * 0.75 + diagonal * 0.25 + wave).clamp(0.0, 1.0);
            img.put_pixel(x, y, Rgba([mix_u8(top[0], bottom[0], mix), mix_u8(top[1], bottom[1], mix), mix_u8(top[2], bottom[2], mix), 255]));
        }
    }
    img
}

fn add_soft_shapes(img: &mut RgbaImage, rng: &mut SmallRng, palette: ([u8; 3], [u8; 3], [u8; 3])) {
    for _ in 0..6 {
        let x = rng.range_i32(250, CANVAS_SIZE as i32 - 250);
        let y = rng.range_i32(250, CANVAS_SIZE as i32 - 250);
        let radius = rng.range_i32(140, 390);
        let color = if rng.next_bool() { palette.0 } else { palette.1 };
        draw_filled_circle_mut(
            img,
            (x, y),
            radius,
            Rgba([color[0], color[1], color[2], rng.range_u8(34, 82)]),
        );
    }
}

fn draw_symbol(
    img: &mut RgbaImage,
    rng: &mut SmallRng,
    style: usize,
    palette: ([u8; 3], [u8; 3], [u8; 3]),
) {
    let fg = Rgba([palette.2[0], palette.2[1], palette.2[2], 238]);
    let accent = Rgba([palette.1[0], palette.1[1], palette.1[2], 230]);
    let center = (CANVAS_SIZE as i32 / 2, CANVAS_SIZE as i32 / 2);

    match style {
        0 => draw_orbit_symbol(img, center, fg, accent),
        1 => draw_stack_symbol(img, center, fg, accent),
        2 => draw_bolt_symbol(img, center, fg, accent),
        3 => draw_diamond_symbol(img, center, fg, accent),
        _ => draw_node_symbol(img, rng, center, fg, accent),
    }
}

fn draw_orbit_symbol(img: &mut RgbaImage, center: (i32, i32), fg: Rgba<u8>, accent: Rgba<u8>) {
    for ring in [540.0_f32, 380.0, 220.0] {
        let half = ring / 2.0;
        draw_polyline(img, ellipse_points(center, half, half * 0.54, -24.0), fg, 22.0);
        draw_polyline(img, ellipse_points(center, half, half * 0.54, 34.0), accent, 14.0);
    }
    draw_filled_circle_mut(img, center, 152, fg);
    draw_filled_circle_mut(img, center, 82, accent);
}

fn draw_stack_symbol(img: &mut RgbaImage, center: (i32, i32), fg: Rgba<u8>, accent: Rgba<u8>) {
    for i in 0..4 {
        let y = center.1 - 330 + i * 220;
        let offset = i % 2 * 70 - 35;
        let points = vec![
            Point::new(center.0 - 360 + offset, y),
            Point::new(center.0 + offset, y - 150),
            Point::new(center.0 + 360 + offset, y),
            Point::new(center.0 + offset, y + 150),
        ];
        draw_polygon_mut(img, &points, if i % 2 == 0 { fg } else { accent });
    }
}

fn draw_bolt_symbol(img: &mut RgbaImage, center: (i32, i32), fg: Rgba<u8>, accent: Rgba<u8>) {
    let bolt = vec![
        Point::new(center.0 + 80, center.1 - 620),
        Point::new(center.0 - 360, center.1 + 40),
        Point::new(center.0 - 60, center.1 + 40),
        Point::new(center.0 - 150, center.1 + 620),
        Point::new(center.0 + 390, center.1 - 100),
        Point::new(center.0 + 80, center.1 - 100),
    ];
    draw_polygon_mut(img, &bolt, fg);
    draw_filled_circle_mut(img, (center.0 + 240, center.1 - 360), 120, accent);
    draw_filled_circle_mut(img, (center.0 - 280, center.1 + 340), 150, accent);
}

fn draw_diamond_symbol(img: &mut RgbaImage, center: (i32, i32), fg: Rgba<u8>, accent: Rgba<u8>) {
    let outer = vec![
        Point::new(center.0, center.1 - 660),
        Point::new(center.0 + 560, center.1),
        Point::new(center.0, center.1 + 660),
        Point::new(center.0 - 560, center.1),
    ];
    let inner = vec![
        Point::new(center.0, center.1 - 370),
        Point::new(center.0 + 310, center.1),
        Point::new(center.0, center.1 + 370),
        Point::new(center.0 - 310, center.1),
    ];
    draw_polygon_mut(img, &outer, fg);
    draw_polygon_mut(img, &inner, accent);
}

fn draw_node_symbol(
    img: &mut RgbaImage,
    rng: &mut SmallRng,
    center: (i32, i32),
    fg: Rgba<u8>,
    accent: Rgba<u8>,
) {
    let mut points = Vec::new();
    for i in 0..6 {
        let angle = i as f32 / 6.0 * std::f32::consts::TAU + rng.range_f32(-0.14, 0.14);
        let radius = rng.range_f32(360.0, 610.0);
        points.push((
            center.0 as f32 + angle.cos() * radius,
            center.1 as f32 + angle.sin() * radius,
        ));
    }
    for i in 0..points.len() {
        draw_thick_line(img, points[i], points[(i + 2) % points.len()], accent, 18.0);
    }
    for point in points {
        draw_filled_circle_mut(img, (point.0 as i32, point.1 as i32), 105, fg);
        draw_filled_circle_mut(img, (point.0 as i32, point.1 as i32), 48, accent);
    }
}

fn draw_logo_text(img: &mut RgbaImage, text: &str, color: [u8; 3]) -> Result<()> {
    let font = FontVec::try_from_vec(EMBEDDED_FONT.to_vec())
        .map_err(|_| anyhow::anyhow!("Failed to load embedded font"))?;
    let size = if text.chars().count() <= 2 { 620.0 } else { 460.0 };
    let scale = PxScale::from(size);
    let width = measure_text_width(&font, scale, text);
    let x = ((CANVAS_SIZE as f32 - width) / 2.0).round() as i32;
    let y = (CANVAS_SIZE as f32 * 0.5 - size * 0.58).round() as i32;

    for (dx, dy) in [
        (-18, 0),
        (18, 0),
        (0, -18),
        (0, 18),
        (-14, -14),
        (14, -14),
        (-14, 14),
        (14, 14),
    ] {
        draw_text_mut(img, Rgba([0, 0, 0, 70]), x + dx, y + dy, scale, &font, text);
    }
    draw_text_mut(
        img,
        Rgba([color[0], color[1], color[2], 255]),
        x,
        y,
        scale,
        &font,
        text,
    );
    Ok(())
}

fn clean_logo_text(text: &str) -> Option<String> {
    let cleaned: String = text
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .take(4)
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn measure_text_width(font: &FontVec, scale: PxScale, text: &str) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut width = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        if let Some(prev_id) = prev {
            width += scaled.kern(prev_id, glyph_id);
        }
        width += scaled.h_advance(glyph_id);
        prev = Some(glyph_id);
    }
    width
}

fn ellipse_points(center: (i32, i32), rx: f32, ry: f32, rotate_deg: f32) -> Vec<(f32, f32)> {
    let angle = rotate_deg.to_radians();
    let cos_r = angle.cos();
    let sin_r = angle.sin();
    (0..96)
        .map(|i| {
            let t = i as f32 / 96.0 * std::f32::consts::TAU;
            let x = t.cos() * rx;
            let y = t.sin() * ry;
            (
                center.0 as f32 + x * cos_r - y * sin_r,
                center.1 as f32 + x * sin_r + y * cos_r,
            )
        })
        .collect()
}

fn draw_polyline(img: &mut RgbaImage, points: Vec<(f32, f32)>, color: Rgba<u8>, width: f32) {
    for i in 0..points.len() {
        draw_thick_line(img, points[i], points[(i + 1) % points.len()], color, width);
    }
}

fn draw_thick_line(
    img: &mut RgbaImage,
    start: (f32, f32),
    end: (f32, f32),
    color: Rgba<u8>,
    width: f32,
) {
    let steps = width.round() as i32;
    let half = steps / 2;
    for dx in -half..=half {
        for dy in -half..=half {
            if dx * dx + dy * dy <= half * half {
                draw_line_segment_mut(
                    img,
                    (start.0 + dx as f32, start.1 + dy as f32),
                    (end.0 + dx as f32, end.1 + dy as f32),
                    color,
                );
            }
        }
    }
}

fn mix_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

struct SmallRng {
    state: u64,
}

impl SmallRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    fn range_usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }

    fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        min + (self.next_u64() % (max - min) as u64) as i32
    }

    fn range_u8(&mut self, min: u8, max: u8) -> u8 {
        min + (self.next_u64() % (max - min) as u64) as u8
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        let unit = (self.next_u64() >> 11) as f32 / ((1_u64 << 53) as f32);
        min + (max - min) * unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_logo_text_to_short_wordmark() {
        assert_eq!(clean_logo_text("  My App  "), Some("MyAp".to_string()));
        assert_eq!(clean_logo_text("   "), None);
    }

    #[test]
    fn renders_deterministic_logo_size() {
        let logo = render_logo(42, Some("L")).unwrap();
        assert_eq!(logo.dimensions(), (OUTPUT_SIZE, OUTPUT_SIZE));
    }
}
