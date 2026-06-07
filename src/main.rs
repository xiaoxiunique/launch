mod clipboard;
mod collage;
mod config;
mod contents_json;
mod device;
mod generate_logo;
mod mockup;
mod output_image;
mod record;
mod resize;
mod screenshot;
mod tts;
mod video_title;
mod watch;
mod zip;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use config::Platform;
use zip::ZipEntry;

/// Check if a command exists in PATH.
pub(crate) fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Parser)]
#[command(name = "launch", about = "App launch toolkit — icons, mockups, and more")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    // --- Backward-compat: allow flat args for icon generation ---
    /// Source image path (for icon generation without subcommand)
    #[arg(global = false)]
    input: Option<PathBuf>,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long, value_delimiter = ',')]
    platforms: Option<Vec<Platform>>,

    #[arg(long)]
    android_filename: Option<String>,

    #[arg(long)]
    no_stores: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Generate app icons for all platforms
    #[command(alias = "i")]
    Icon(IconArgs),
    /// Generate starter assets
    #[command(alias = "g")]
    Generate(GenerateArgs),
    /// Wrap screenshot in a device frame mockup
    #[command(alias = "m")]
    Mockup(MockupArgs),
    /// Capture screenshot from iOS Simulator and apply mockup
    #[command(alias = "c")]
    Capture(CaptureArgs),
    /// Capture screenshot from Android device via ADB
    #[command(alias = "ac")]
    Acapture(AcaptureArgs),
    /// Capture a macOS app window screenshot
    #[command(alias = "wc")]
    Wcapture(WcaptureArgs),
    /// Generate App Store screenshot with title and device mockup
    #[command(alias = "s")]
    Screenshot(ScreenshotArgs),
    /// Combine multiple screenshots into a single collage image
    #[command(alias = "co")]
    Collage(CollageArgs),
    /// Watch iOS Simulator and auto-capture on screen changes
    #[command(alias = "w")]
    Watch(WatchArgs),
    /// Watch Android device and auto-capture on screen changes
    #[command(alias = "aw")]
    Awatch(AwatchArgs),
    /// Record iOS Simulator video and optionally cut it into clips
    #[command(alias = "r")]
    Record(RecordArgs),
    /// Apply a device frame to an existing video
    #[command(alias = "fv")]
    FrameVideo(FrameVideoArgs),
    /// Add large cover-style title text to a video
    #[command(alias = "t")]
    Title(TitleArgs),
    /// Generate speech from text with Doubao TTS
    #[command(alias = "say")]
    Tts(Box<TtsArgs>),
}

#[derive(Parser)]
struct IconArgs {
    /// Source image path (1024x1024 recommended)
    input: PathBuf,

    /// Output ZIP file path
    #[arg(short, long, default_value = "AppIcons.zip")]
    output: PathBuf,

    /// Platforms to generate icons for (comma-separated)
    #[arg(short, long, value_delimiter = ',', default_value = "all")]
    platforms: Vec<Platform>,

    /// Custom filename for Android icons
    #[arg(long, default_value = "ic_launcher.png")]
    android_filename: String,

    /// Skip App Store and Play Store icon generation
    #[arg(long)]
    no_stores: bool,
}

#[derive(Parser)]
struct GenerateArgs {
    #[command(subcommand)]
    command: GenerateCommand,
}

#[derive(Subcommand)]
enum GenerateCommand {
    /// Generate a random logo and app icon ZIP
    #[command(alias = "l")]
    Logo(GenerateLogoArgs),
}

#[derive(Parser)]
struct GenerateLogoArgs {
    /// Output directory for generated assets
    #[arg(short, long, default_value = ".")]
    output: PathBuf,

    /// Base filename for the generated logo PNG
    #[arg(long, default_value = "logo.png")]
    logo_name: String,

    /// Output ZIP file path. Defaults to <output>/AppIcons.zip.
    #[arg(short, long)]
    icons: Option<PathBuf>,

    /// Optional short text to draw in the logo, such as app initials
    #[arg(short, long)]
    text: Option<String>,

    /// Deterministic seed for repeatable logo generation
    #[arg(long)]
    seed: Option<u64>,

    /// Platforms to generate icons for (comma-separated)
    #[arg(short, long, value_delimiter = ',', default_value = "all")]
    platforms: Vec<Platform>,

    /// Custom filename for Android icons
    #[arg(long, default_value = "ic_launcher.png")]
    android_filename: String,

    /// Skip App Store and Play Store icon generation
    #[arg(long)]
    no_stores: bool,
}

#[derive(Parser)]
struct MockupArgs {
    /// Screenshot image to wrap in device frame
    input: Option<PathBuf>,

    /// Output PNG file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Read screenshot from clipboard
    #[arg(short, long)]
    clipboard: bool,

    /// List available devices and exit
    #[arg(long)]
    list_devices: bool,
}

#[derive(Parser)]
struct CaptureArgs {
    /// Output filename
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Save raw screenshot too (without mockup frame)
    #[arg(long)]
    raw: bool,

    /// Title text — when set, generates a full App Store screenshot
    #[arg(short, long)]
    title: Option<String>,

    /// Font size for title (used with --title)
    #[arg(long, default_value = "200")]
    font_size: f32,

    /// Custom font file for title (used with --title)
    #[arg(long)]
    font: Option<PathBuf>,
}

#[derive(Parser)]
struct AcaptureArgs {
    /// Output filename
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// ADB device serial (when multiple devices connected)
    #[arg(short, long)]
    serial: Option<String>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_ANDROID_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Save raw screenshot too (without processing)
    #[arg(long)]
    raw: bool,

    /// Title text — when set, generates a full Play Store screenshot
    #[arg(short, long)]
    title: Option<String>,

    /// Font size for title (used with --title)
    #[arg(long, default_value = "200")]
    font_size: f32,

    /// Custom font file for title (used with --title)
    #[arg(long)]
    font: Option<PathBuf>,
}

#[derive(Parser)]
struct WcaptureArgs {
    /// App name to capture (e.g. "Simulator", "Safari", "微信")
    app: String,

    /// Output filename
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Save raw screenshot too (without processing)
    #[arg(long)]
    raw: bool,

    /// Device frame ID (auto: iPhone for Simulator, MacBook for other apps)
    #[arg(short, long)]
    device: Option<String>,

    /// Orientation
    #[arg(long)]
    orientation: Option<String>,

    /// Title text — when set, generates a full screenshot with title
    #[arg(short, long)]
    title: Option<String>,

    /// Font size for title (used with --title)
    #[arg(long, default_value = "200")]
    font_size: f32,

    /// Custom font file for title (used with --title)
    #[arg(long)]
    font: Option<PathBuf>,

    /// List windows for the app and exit
    #[arg(long)]
    list: bool,

    /// Skip device mockup frame (just raw window capture)
    #[arg(long)]
    no_mockup: bool,
}

#[derive(Parser)]
struct ScreenshotArgs {
    /// Screenshot image (or directory for batch processing)
    input: PathBuf,

    /// Title text to display above the device mockup
    #[arg(short, long)]
    title: String,

    /// Output file or directory path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Custom font file (.ttf/.otf/.ttc)
    #[arg(long)]
    font: Option<PathBuf>,

    /// Font size in pixels
    #[arg(long, default_value = "200")]
    font_size: f32,
}

#[derive(Parser)]
struct CollageArgs {
    /// Directory containing screenshots (defaults to current directory)
    #[arg(default_value = ".")]
    input: PathBuf,

    /// Output file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Padding between images in pixels (auto-scaled if not set)
    #[arg(long)]
    padding: Option<u32>,

    /// Skip device frame mockup (use raw screenshots)
    #[arg(long)]
    no_frame: bool,
}

#[derive(Parser)]
struct WatchArgs {
    /// Output directory for captured screenshots
    #[arg(short, long, default_value = "./watch-screenshots")]
    output: PathBuf,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Polling interval in seconds
    #[arg(long, default_value = "1.0")]
    interval: f32,

    /// Skip collage generation on exit
    #[arg(long)]
    no_collage: bool,
}

#[derive(Parser)]
struct AwatchArgs {
    /// Output directory for captured screenshots
    #[arg(short, long, default_value = "./awatch-screenshots")]
    output: PathBuf,

    /// ADB device serial (when multiple devices connected)
    #[arg(short, long)]
    serial: Option<String>,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_ANDROID_DEVICE)]
    device: String,

    /// Orientation: portrait or landscape
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Polling interval in seconds
    #[arg(long, default_value = "1.0")]
    interval: f32,

    /// Skip collage generation on exit
    #[arg(long)]
    no_collage: bool,
}

#[derive(Parser)]
struct RecordArgs {
    /// Output directory for recording sessions
    #[arg(short, long, default_value = "./recordings")]
    output: PathBuf,

    /// Session folder name (defaults to recording-<timestamp>)
    #[arg(long)]
    name: Option<String>,

    /// Apply a device frame to the exported video or clips
    #[arg(long)]
    frame: bool,

    /// Device frame ID used with --frame
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Device orientation used with --frame
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Outer padding around the framed device video
    #[arg(long, default_value = "96")]
    frame_padding: u32,

    /// Detect and remove repeated still video inside clips
    #[arg(long)]
    auto_trim: bool,

    /// Minimum still duration, in seconds, before auto-trim removes it
    #[arg(long, default_value = "2.0")]
    freeze_min: f64,

    /// FFmpeg freezedetect noise tolerance, from 0 to 1
    #[arg(long, default_value = "0.001")]
    freeze_noise: f64,

    /// Seconds of still content to keep at each side of a removed interval
    #[arg(long, default_value = "0.4")]
    keep_still: f64,

    /// Write markers/cut plan without exporting trimmed clips
    #[arg(long)]
    dry_run: bool,
}

#[derive(Parser)]
struct FrameVideoArgs {
    /// Source video to wrap in a device frame
    input: PathBuf,

    /// Output MP4 path
    #[arg(short, long)]
    output: PathBuf,

    /// Device frame ID
    #[arg(short, long, default_value = device::DEFAULT_DEVICE)]
    device: String,

    /// Device orientation
    #[arg(long, default_value = "portrait")]
    orientation: String,

    /// Outer padding around the framed device video
    #[arg(long, default_value = "96")]
    padding: u32,
}

#[derive(Parser)]
struct TitleArgs {
    /// Source video to add title text to
    input: PathBuf,

    /// Title text. Use "\n" for manual line breaks; long text wraps automatically.
    #[arg(short, long)]
    text: String,

    /// Output MP4 path
    #[arg(short, long)]
    output: PathBuf,

    /// Seconds to show the title overlay from the start
    #[arg(long, default_value = "1.0")]
    duration: f64,

    /// Vertical title center as a ratio of video height
    #[arg(long, default_value = "0.40")]
    y_ratio: f64,

    /// Maximum title line width as a ratio of video width
    #[arg(long, default_value = "0.78")]
    wrap_width: f64,

    /// Font size in pixels. Auto-sized when omitted.
    #[arg(long)]
    font_size: Option<u32>,

    /// Custom font file
    #[arg(long)]
    font: Option<PathBuf>,
}

#[derive(Parser)]
struct TtsArgs {
    #[command(subcommand)]
    command: Option<TtsCommand>,

    /// Text to synthesize
    text: Option<String>,

    /// Read text from a UTF-8 file
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output audio path
    #[arg(short, long, default_value_os_t = tts::default_output())]
    output: PathBuf,

    /// Doubao voice ID
    #[arg(long, default_value_t = tts::default_voice())]
    voice: String,

    /// TTS model name
    #[arg(long, default_value_t = tts::default_model())]
    model: String,

    /// Audio format, such as mp3, wav, opus, or pcm
    #[arg(long, default_value_t = tts::default_format())]
    format: String,

    /// Speech speed
    #[arg(long, default_value = "1.0")]
    speed: f32,

    /// TTS provider: gateway for API Key or volcengine for legacy App ID credentials
    #[arg(long, value_enum, default_value = "gateway")]
    provider: tts::TtsProvider,

    /// Volcengine API Key TTS endpoint
    #[arg(long, default_value_t = tts::default_base_url())]
    base_url: String,

    /// Volcengine legacy App ID TTS endpoint
    #[arg(long, default_value_t = tts::default_volcengine_url())]
    volcengine_url: String,

    /// Environment variable used for the API key
    #[arg(long, default_value_t = tts::default_api_key_env())]
    api_key_env: String,

    /// Volcengine App ID for --provider volcengine
    #[arg(long)]
    app_id: Option<String>,

    /// Volcengine App Key for --provider volcengine
    #[arg(long, default_value_t = tts::default_app_key())]
    app_key: String,

    /// Volcengine Access Key for --provider volcengine
    #[arg(long)]
    access_key: Option<String>,

    /// Volcengine resource ID
    #[arg(long, default_value_t = tts::default_resource_id())]
    resource_id: String,

    /// Play audio after generation using afplay
    #[arg(long)]
    play: bool,
}

#[derive(Subcommand)]
enum TtsCommand {
    /// Save the Doubao TTS API key and default settings
    Config(TtsConfigArgs),
}

#[derive(Parser)]
struct TtsConfigArgs {
    /// Volcengine API key to save locally
    #[arg(long)]
    api_key: Option<String>,

    /// Default Volcengine API Key TTS endpoint
    #[arg(long)]
    base_url: Option<String>,

    /// Default TTS provider: gateway or volcengine
    #[arg(long, value_enum)]
    provider: Option<tts::TtsProvider>,

    /// Volcengine App ID
    #[arg(long)]
    app_id: Option<String>,

    /// Volcengine App Key
    #[arg(long)]
    app_key: Option<String>,

    /// Volcengine Access Key
    #[arg(long)]
    access_key: Option<String>,

    /// Volcengine resource ID
    #[arg(long)]
    resource_id: Option<String>,

    /// Default Doubao voice ID
    #[arg(long)]
    voice: Option<String>,

    /// Default TTS model
    #[arg(long)]
    model: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Icon(args)) => run_icon(args),
        Some(Command::Generate(args)) => run_generate(args),
        Some(Command::Mockup(args)) => run_mockup(args),
        Some(Command::Capture(args)) => run_capture(args),
        Some(Command::Acapture(args)) => run_acapture(args),
        Some(Command::Wcapture(args)) => run_wcapture(args),
        Some(Command::Screenshot(args)) => run_screenshot(args),
        Some(Command::Collage(args)) => run_collage(args),
        Some(Command::Watch(args)) => run_watch(args),
        Some(Command::Awatch(args)) => run_awatch(args),
        Some(Command::Record(args)) => run_record(args),
        Some(Command::FrameVideo(args)) => run_frame_video(args),
        Some(Command::Title(args)) => run_title(args),
        Some(Command::Tts(args)) => run_tts(*args),
        None => {
            // Backward compat: treat as icon generation if input is provided
            if let Some(input) = cli.input {
                let args = IconArgs {
                    input,
                    output: cli.output.unwrap_or_else(|| PathBuf::from("AppIcons.zip")),
                    platforms: cli.platforms.unwrap_or_else(|| vec![Platform::All]),
                    android_filename: cli
                        .android_filename
                        .unwrap_or_else(|| "ic_launcher.png".into()),
                    no_stores: cli.no_stores,
                };
                run_icon(args)
            } else {
                // No input, no subcommand — show help
                use clap::CommandFactory;
                Cli::command().print_help()?;
                Ok(())
            }
        }
    }
}

fn run_icon(args: IconArgs) -> Result<()> {
    generate_icon_zip(
        &args.input,
        &args.output,
        &args.platforms,
        &args.android_filename,
        args.no_stores,
    )
}

fn run_generate(args: GenerateArgs) -> Result<()> {
    match args.command {
        GenerateCommand::Logo(args) => run_generate_logo(args),
    }
}

fn run_generate_logo(args: GenerateLogoArgs) -> Result<()> {
    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("Failed to create {}", args.output.display()))?;

    let logo_path = args.output.join(&args.logo_name);
    let icons_path = args
        .icons
        .unwrap_or_else(|| args.output.join("AppIcons.zip"));

    generate_logo::run(generate_logo::LogoOptions {
        output: logo_path.clone(),
        text: args.text,
        seed: args.seed,
    })?;

    generate_icon_zip(
        &logo_path,
        &icons_path,
        &args.platforms,
        &args.android_filename,
        args.no_stores,
    )?;

    eprintln!("Generated starter logo -> {}", logo_path.display());
    eprintln!("Generated app icons -> {}", icons_path.display());
    Ok(())
}

fn generate_icon_zip(
    input: &Path,
    output: &Path,
    platform_args: &[Platform],
    android_filename: &str,
    no_stores: bool,
) -> Result<()> {
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }

    let platforms = config::expand_platforms(platform_args);

    let img = resize::load_image(input)?;

    // Collect unique (size, fill_white_bg) pairs
    let mut size_requests: Vec<(u32, bool)> = Vec::new();

    for &platform in &platforms {
        let fill_white = !config::preserves_alpha(platform);
        let entries = config::get_entries(platform, android_filename);
        for entry in &entries {
            let key = (entry.expected_size, fill_white);
            if !size_requests.contains(&key) {
                size_requests.push(key);
            }
        }
    }

    if !no_stores {
        for entry in &config::store_entries() {
            let fill_white = !config::store_preserves_alpha(&entry.filename);
            let key = (entry.expected_size, fill_white);
            if !size_requests.contains(&key) {
                size_requests.push(key);
            }
        }
    }

    eprintln!(
        "Resizing {} unique icon sizes from {}...",
        size_requests.len(),
        input.display()
    );
    let resized = resize::resize_all(&img, &size_requests)?;

    // Build ZIP entries
    let mut zip_entries: Vec<ZipEntry> = Vec::new();
    let mut icon_count: usize = 0;

    for &platform in &platforms {
        let fill_white = !config::preserves_alpha(platform);
        let entries = config::get_entries(platform, android_filename);
        for entry in &entries {
            let data = resized[&(entry.expected_size, fill_white)].clone();
            let path = format!("{}{}", entry.folder, entry.filename);
            zip_entries.push(ZipEntry { path, data });
            icon_count += 1;
        }
    }

    if !no_stores {
        for entry in &config::store_entries() {
            let fill_white = !config::store_preserves_alpha(&entry.filename);
            let data = resized[&(entry.expected_size, fill_white)].clone();
            let path = entry.filename.clone();
            zip_entries.push(ZipEntry { path, data });
            icon_count += 1;
        }
    }

    let has_apple = platforms.iter().any(|p| config::is_apple_platform(*p));
    if has_apple {
        let contents = contents_json::generate(&platforms, android_filename);
        zip_entries.push(ZipEntry {
            path: "Assets.xcassets/AppIcon.appiconset/Contents.json".into(),
            data: contents.into_bytes(),
        });
    }

    zip::build_zip(output, zip_entries)?;

    eprintln!(
        "Generated {} icons for {} platform(s) -> {}",
        icon_count,
        platforms.len(),
        output.display()
    );

    Ok(())
}

fn run_mockup(args: MockupArgs) -> Result<()> {
    if args.list_devices {
        mockup::list_devices();
        return Ok(());
    }

    let input = if args.clipboard {
        let path = clipboard::save_clipboard_image()?;
        eprintln!("Read image from clipboard");
        path
    } else {
        args.input.ok_or_else(|| {
            anyhow::anyhow!(
                "Provide a screenshot path, directory, or use -c to read from clipboard.\n\
                 Usage: launch mockup <screenshot.png>\n\
                        launch mockup <screenshots_dir/>\n\
                        launch mockup -c"
            )
        })?
    };

    // If input is a directory, batch process all images
    if input.is_dir() {
        let out_dir = args.output.unwrap_or_else(|| input.join("mockups"));
        std::fs::create_dir_all(&out_dir)?;

        let mut count = 0;
        for entry in std::fs::read_dir(&input)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                continue;
            }
            if mockup::is_already_processed(&path) {
                eprintln!("Skipping {} (already processed)", path.display());
                continue;
            }
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_path = out_dir.join(format!("{}-mockup.png", stem));
            mockup::run(&path, &out_path, &args.device, &args.orientation)?;
            count += 1;
        }
        eprintln!("Processed {} images -> {}", count, out_dir.display());
        return Ok(());
    }

    if mockup::is_already_processed(&input) {
        anyhow::bail!("Image already has mockup frame: {}", input.display());
    }

    let output = args.output.unwrap_or_else(|| {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("{}-mockup.png", stem))
    });

    mockup::run(&input, &output, &args.device, &args.orientation)
}

fn run_capture(args: CaptureArgs) -> Result<()> {
    use std::process::Command as Cmd;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Generate timestamp for filename
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let raw_path = std::env::temp_dir().join(format!("launch-capture-{}.png", ts));

    // Capture from booted simulator
    eprintln!("Capturing screenshot from iOS Simulator...");
    let status = Cmd::new("xcrun")
        .args(["simctl", "io", "booted", "screenshot", "--type=png"])
        .arg(&raw_path)
        .status()
        .context("Failed to run xcrun simctl. Is Xcode installed?")?;

    if !status.success() {
        anyhow::bail!(
            "Simulator screenshot failed. Make sure a simulator is running.\n\
             Start one with: open -a Simulator"
        );
    }

    eprintln!("Captured simulator screenshot");

    // Determine output path
    let has_title = args.title.is_some();
    let default_suffix = if has_title { "screenshot" } else { "mockup" };
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!("screenshot-{}-{}.png", ts, default_suffix))
    });

    // Optionally keep raw screenshot
    if args.raw {
        let raw_out = output
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("screenshot-{}-raw.png", ts));
        std::fs::copy(&raw_path, &raw_out)?;
        eprintln!("Raw screenshot saved to {}", raw_out.display());
    }

    if let Some(title) = &args.title {
        // Full App Store screenshot: capture → mockup → title
        screenshot::run(
            &raw_path,
            &output,
            title,
            &args.device,
            &args.orientation,
            args.font.as_deref(),
            args.font_size,
        )?;
    } else {
        // Just mockup
        mockup::run(&raw_path, &output, &args.device, &args.orientation)?;
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&raw_path);

    Ok(())
}

fn run_acapture(args: AcaptureArgs) -> Result<()> {
    use std::process::Command as Cmd;
    use std::time::{SystemTime, UNIX_EPOCH};

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let raw_path = std::env::temp_dir().join(format!("launch-acapture-{}.png", ts));

    // Find adb binary — check PATH first, then common install locations
    let adb = {
        let common_paths = [
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join("Library/Android/sdk/platform-tools/adb")),
            Some(PathBuf::from("/opt/homebrew/bin/adb")),
            Some(PathBuf::from("/usr/local/bin/adb")),
        ];
        if which_exists("adb") {
            PathBuf::from("adb")
        } else if let Some(path) = common_paths.iter().flatten().find(|p| p.exists()) {
            eprintln!("Found adb at {}", path.display());
            path.clone()
        } else {
            anyhow::bail!(
                "adb not found. Install Android SDK or add platform-tools to PATH.\n\
                 Common location: ~/Library/Android/sdk/platform-tools/"
            );
        }
    };

    // Auto-detect ADB device if not specified
    let serial = if let Some(s) = args.serial {
        s
    } else {
        let output = Cmd::new(&adb)
            .args(["devices"])
            .output()
            .context("Failed to run adb. Is Android SDK installed?")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<&str> = stdout
            .lines()
            .skip(1) // skip "List of devices attached"
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "device" {
                    Some(parts[0])
                } else {
                    None
                }
            })
            .collect();

        match devices.len() {
            0 => anyhow::bail!(
                "No Android device found.\n\
                 Connect a device via USB or start an emulator, then check: adb devices"
            ),
            1 => {
                eprintln!("Detected device: {}", devices[0]);
                devices[0].to_string()
            }
            _ => {
                eprintln!("Multiple devices detected:");
                for d in &devices {
                    eprintln!("  {}", d);
                }
                eprintln!("Using first device: {}", devices[0]);
                eprintln!("Tip: use -s <serial> to specify a device");
                devices[0].to_string()
            }
        }
    };

    // Capture from Android device via ADB
    eprintln!("Capturing screenshot from {}...", serial);
    let output_data = Cmd::new(&adb)
        .args(["-s", &serial, "exec-out", "screencap", "-p"])
        .output()
        .context("Failed to run adb screencap")?;

    if !output_data.status.success() {
        anyhow::bail!(
            "ADB screenshot failed for device {}.\n\
             Check with: adb devices",
            serial
        );
    }

    std::fs::write(&raw_path, &output_data.stdout)
        .context("Failed to write screenshot")?;

    eprintln!("Captured Android screenshot");

    // Determine output path
    let has_title = args.title.is_some();
    let default_suffix = if has_title { "screenshot" } else { "mockup" };
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!("android-{}-{}.png", ts, default_suffix))
    });

    // Optionally keep raw screenshot
    if args.raw {
        let raw_out = output
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("android-{}-raw.png", ts));
        std::fs::copy(&raw_path, &raw_out)?;
        eprintln!("Raw screenshot saved to {}", raw_out.display());
    }

    if let Some(title) = &args.title {
        // Full Play Store screenshot: capture → mockup → title
        screenshot::run_android(
            &raw_path,
            &output,
            title,
            &args.device,
            &args.orientation,
            args.font.as_deref(),
            args.font_size,
        )?;
    } else {
        // Just mockup
        mockup::run(&raw_path, &output, &args.device, &args.orientation)?;
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&raw_path);

    Ok(())
}

fn run_wcapture(args: WcaptureArgs) -> Result<()> {
    use std::process::Command as Cmd;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Find window IDs using Swift + CoreGraphics
    let swift_code = format!(
        r#"
        import CoreGraphics
        let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String: Any]]
        for w in windows {{
            let name = w["kCGWindowOwnerName"] as? String ?? ""
            let title = w["kCGWindowName"] as? String ?? ""
            let wid = w["kCGWindowNumber"] as? Int ?? 0
            let bounds = w["kCGWindowBounds"] as? [String: Any] ?? [:]
            let ww = bounds["Width"] as? Int ?? 0
            let wh = bounds["Height"] as? Int ?? 0
            if name.localizedCaseInsensitiveContains("{}") && ww > 100 && wh > 100 {{
                print("\(wid)|\(name)|\(title)|\(ww)x\(wh)")
            }}
        }}
        "#,
        args.app
    );

    let output = Cmd::new("swift")
        .args(["-e", &swift_code])
        .output()
        .context("Failed to run swift")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list windows: {}", err);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let windows: Vec<&str> = stdout.trim().lines().filter(|l| !l.is_empty()).collect();

    if windows.is_empty() {
        anyhow::bail!(
            "No window found for \"{}\". Make sure the app is running and has a visible window.",
            args.app
        );
    }

    if args.list {
        eprintln!("Windows matching \"{}\":", args.app);
        for w in &windows {
            let parts: Vec<&str> = w.splitn(4, '|').collect();
            if parts.len() >= 4 {
                eprintln!("  ID: {}  App: {}  Title: {}  Size: {}", parts[0], parts[1], parts[2], parts[3]);
            }
        }
        return Ok(());
    }

    // Use the first matching window
    let parts: Vec<&str> = windows[0].splitn(4, '|').collect();
    let window_id = parts[0];
    let app_name = parts.get(1).unwrap_or(&"");
    let window_title = parts.get(2).unwrap_or(&"");

    if windows.len() > 1 {
        eprintln!("Found {} windows for \"{}\", using first:", windows.len(), args.app);
    }
    eprintln!("Capturing: {} - {} (ID: {})", app_name, window_title, window_id);

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let raw_path = std::env::temp_dir().join(format!("launch-wcapture-{}.png", ts));

    // For Simulator, use xcrun simctl for a clean screenshot (no window chrome)
    let is_simulator = app_name.to_lowercase().contains("simulator");
    if is_simulator {
        eprintln!("Detected Simulator — using simctl for clean capture");
        let status = Cmd::new("xcrun")
            .args(["simctl", "io", "booted", "screenshot", "--type=png"])
            .arg(&raw_path)
            .status()
            .context("Failed to run xcrun simctl")?;
        if !status.success() {
            anyhow::bail!("Simulator screenshot failed. Is a simulator booted?");
        }
    } else {
        // Generic window capture (-o: no shadow, -x: no sound)
        let status = Cmd::new("screencapture")
            .args(["-l", window_id, "-o", "-x"])
            .arg(&raw_path)
            .status()
            .context("Failed to run screencapture")?;
        if !status.success() {
            anyhow::bail!("screencapture failed for window ID {}", window_id);
        }
    }

    eprintln!("Captured window screenshot");

    // Auto-select device: Simulator → iPhone, other Mac apps → MacBook
    let device_id = args.device.unwrap_or_else(|| {
        if is_simulator {
            device::DEFAULT_DEVICE.to_string()
        } else {
            device::DEFAULT_MAC_DEVICE.to_string()
        }
    });
    let orientation = args.orientation.unwrap_or_else(|| {
        if is_simulator { "portrait".to_string() } else { "front".to_string() }
    });

    // Determine output path
    let has_title = args.title.is_some();
    let default_suffix = if has_title { "screenshot" } else if args.no_mockup { "raw" } else { "mockup" };
    let safe_name: String = args.app.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    let output = args.output.unwrap_or_else(|| {
        PathBuf::from(format!("{}-{}-{}.png", safe_name, ts, default_suffix))
    });

    // Optionally keep raw screenshot
    if args.raw {
        let raw_out = output
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{}-{}-raw.png", safe_name, ts));
        std::fs::copy(&raw_path, &raw_out)?;
        eprintln!("Raw screenshot saved to {}", raw_out.display());
    }

    if let Some(title) = &args.title {
        screenshot::run(
            &raw_path,
            &output,
            title,
            &device_id,
            &orientation,
            args.font.as_deref(),
            args.font_size,
        )?;
    } else if args.no_mockup {
        std::fs::copy(&raw_path, &output)?;
        eprintln!("Saved to {}", output.display());
    } else {
        mockup::run(&raw_path, &output, &device_id, &orientation)?;
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&raw_path);

    Ok(())
}

fn run_screenshot(args: ScreenshotArgs) -> Result<()> {
    let font_path = args.font.as_deref();

    if args.input.is_dir() {
        let out_dir = args.output.unwrap_or_else(|| args.input.join("screenshots"));
        std::fs::create_dir_all(&out_dir)?;

        let mut count = 0;
        for entry in std::fs::read_dir(&args.input)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                continue;
            }
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let out_path = out_dir.join(format!("{}-screenshot.png", stem));
            screenshot::run(
                &path,
                &out_path,
                &args.title,
                &args.device,
                &args.orientation,
                font_path,
                args.font_size,
            )?;
            count += 1;
        }
        eprintln!("Processed {} images -> {}", count, out_dir.display());
        return Ok(());
    }

    let output = args.output.unwrap_or_else(|| {
        let stem = args.input.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!("{}-screenshot.png", stem))
    });

    screenshot::run(
        &args.input,
        &output,
        &args.title,
        &args.device,
        &args.orientation,
        font_path,
        args.font_size,
    )
}

fn run_collage(args: CollageArgs) -> Result<()> {
    if !args.input.is_dir() {
        anyhow::bail!(
            "Expected a directory of screenshots.\n\
             Usage: launch collage <screenshots_dir/>"
        );
    }

    // Collect image files from directory, sorted by name
    let mut images: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&args.input)
        .with_context(|| format!("Failed to read directory: {}", args.input.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            images.push(path);
        }
    }
    images.sort();

    if images.is_empty() {
        anyhow::bail!(
            "No images found in {}. Supported formats: png, jpg, jpeg, webp",
            args.input.display()
        );
    }

    let output = args.output.unwrap_or_else(|| {
        let dir_name = args
            .input
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        PathBuf::from(format!("{}-collage.png", dir_name))
    });

    collage::run(
        &images,
        &output,
        &args.device,
        &args.orientation,
        args.padding,
        args.no_frame,
    )
}

fn run_watch(args: WatchArgs) -> Result<()> {
    let interval = std::time::Duration::from_secs_f32(args.interval);
    watch::run(
        &args.output,
        &args.device,
        &args.orientation,
        interval,
        args.no_collage,
    )
}

fn run_awatch(args: AwatchArgs) -> Result<()> {
    let interval = std::time::Duration::from_secs_f32(args.interval);
    watch::run_android(
        &args.output,
        &args.device,
        &args.orientation,
        interval,
        args.no_collage,
        args.serial.as_deref(),
    )
}

fn run_record(args: RecordArgs) -> Result<()> {
    if args.freeze_min <= 0.0 {
        anyhow::bail!("--freeze-min must be greater than 0");
    }
    if !(0.0..=1.0).contains(&args.freeze_noise) {
        anyhow::bail!("--freeze-noise must be between 0 and 1");
    }
    if args.keep_still < 0.0 {
        anyhow::bail!("--keep-still must be 0 or greater");
    }
    if args.frame {
        validate_frame_device(&args.device, &args.orientation)?;
    }

    record::run(record::RecordOptions {
        output_dir: args.output,
        name: args.name,
        frame: args.frame,
        frame_device: args.device,
        frame_orientation: args.orientation,
        frame_padding: args.frame_padding,
        auto_trim: args.auto_trim,
        freeze_min: args.freeze_min,
        freeze_noise: args.freeze_noise,
        keep_still: args.keep_still,
        dry_run: args.dry_run,
    })
}

fn run_frame_video(args: FrameVideoArgs) -> Result<()> {
    validate_frame_device(&args.device, &args.orientation)?;
    if !args.input.exists() {
        anyhow::bail!("Input video not found: {}", args.input.display());
    }
    if let Some(parent) = args.output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }

    record::frame_existing_video(record::FrameVideoOptions {
        input: args.input,
        output: args.output,
        device: args.device,
        orientation: args.orientation,
        padding: args.padding,
    })
}

fn run_title(args: TitleArgs) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("Input video not found: {}", args.input.display());
    }
    if let Some(parent) = args.output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
    }

    video_title::run(video_title::TitleOptions {
        input: args.input,
        output: args.output,
        text: args.text,
        duration: args.duration,
        y_ratio: args.y_ratio,
        wrap_width: args.wrap_width,
        font_size: args.font_size,
        font: args.font,
    })
}

fn run_tts(args: TtsArgs) -> Result<()> {
    match args.command {
        Some(TtsCommand::Config(args)) => tts::configure(tts::TtsConfigOptions {
            api_key: args.api_key,
            base_url: args.base_url,
            provider: args.provider,
            app_id: args.app_id,
            app_key: args.app_key,
            access_key: args.access_key,
            resource_id: args.resource_id,
            voice: args.voice,
            model: args.model,
        }),
        None => tts::run(tts::TtsOptions {
            text: args.text,
            file: args.file,
            output: args.output,
            voice: args.voice,
            model: args.model,
            format: args.format,
            speed: args.speed,
            provider: args.provider,
            base_url: args.base_url,
            volcengine_url: args.volcengine_url,
            api_key_env: args.api_key_env,
            app_id: args.app_id,
            app_key: args.app_key,
            access_key: args.access_key,
            resource_id: args.resource_id,
            play: args.play,
        }),
    }
}

fn validate_frame_device(device_id: &str, orientation: &str) -> Result<()> {
    let frame_device = device::find_device(device_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown device: {}", device_id))?;
    if frame_device.find_orientation(orientation).is_none() {
        anyhow::bail!(
            "Device {} does not support orientation: {}",
            device_id,
            orientation
        );
    }
    Ok(())
}
