#![forbid(unsafe_code)]

mod api;
mod browser;
mod config;
mod downloader;
mod mango;
mod models;
mod player;
mod tui;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use downloader::Downloader;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "suno-tui",
    author = "pilldev",
    version = "0.1.0",
    about = "A powerful, feature-rich TUI and CLI for browsing and downloading Suno.ai tracks"
)]
struct CliArgs {
    /// Suno song URLs or UUIDs to download or inspect
    #[arg(value_name = "URL_OR_UUID")]
    urls: Vec<String>,

    /// Headless CLI mode: download specified URLs directly without launching TUI
    #[arg(short = 'b', long = "cli")]
    cli_mode: bool,

    /// Suno Clerk session token (Bearer token or Cookie)
    #[arg(short = 't', long = "token")]
    token: Option<String>,

    /// Custom output directory for downloaded audio files
    #[arg(short = 'o', long = "output")]
    output_dir: Option<PathBuf>,

    /// Embed ID3 tags (lyrics, cover art, artist, style) into MP3
    #[arg(long = "embed-id3", default_value_t = true)]
    embed_id3: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    // Load config
    let mut config = Config::load();
    if let Some(token) = args.token {
        config.session_token = Some(token);
    }
    if let Some(out_dir) = args.output_dir {
        config.download_dir = out_dir;
    }
    config.embed_id3 = args.embed_id3;

    // If CLI mode was requested or URLs given with --cli
    if args.cli_mode {
        return run_headless_cli(&config, args.urls).await;
    }

    // Default: run interactive TUI
    run_tui(config, args.urls).await
}

async fn run_headless_cli(config: &Config, urls: Vec<String>) -> Result<()> {
    if urls.is_empty() {
        eprintln!("Error: Headless CLI mode requires at least one URL or UUID.");
        eprintln!("Usage: suno-tui --cli <URL_OR_UUID>...");
        std::process::exit(1);
    }

    println!("🎵 Suno Downloader CLI");
    println!("Output directory: {}", config.download_dir.display());

    let client = api::SunoClient::new(config.session_token.clone())?;
    let downloader = Downloader::new(config.clone());

    for url in urls {
        println!("\n▶ Processing: {}", url);
        match client.fetch_clip(&url).await {
            Ok(clip) => {
                println!("  Title:  {}", clip.display_title());
                println!("  Artist: {}", clip.display_artist());
                println!("  Style:  {}", clip.display_tags());
                println!("  Audio:  {}", clip.get_audio_url());

                println!("  Downloading audio and embedding ID3 tags...");
                match downloader.download_clip(&clip, None).await {
                    Ok(path) => {
                        println!("  ✔ Saved successfully to: {}", path.display());
                    }
                    Err(e) => {
                        eprintln!("  ✖ Failed to download track: {:#}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("  ✖ Failed to resolve Suno track: {:#}", e);
            }
        }
    }

    println!("\nDone.");
    Ok(())
}

async fn run_tui(config: Config, initial_urls: Vec<String>) -> Result<()> {
    // Initialize image picker with intelligent terminal capability detection
    let mut picker = ratatui_image::picker::Picker::from_query_stdio()
        .ok()
        .unwrap_or_else(|| ratatui_image::picker::Picker::from_fontsize((8, 16)));

    // In KDE Konsole, Foot, and WezTerm, Sixel provides crisp, high-resolution graphics.
    // In native Kitty, Kitty graphics protocol is used.
    // Otherwise fallback to universal TrueColor halfblocks.
    let best_proto = detect_best_protocol(&picker);
    picker.set_protocol_type(best_proto);
    let picker = Some(picker);

    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Create app
    let mut app = tui::app::App::new(config, picker);

    // If initial URLs were passed on command line, schedule them for fetching
    for url in initial_urls {
        app.fetch_single_clip(url);
    }

    // Run event loop
    let res = tui::events::run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .ok();
    terminal.show_cursor().ok();

    if let Err(err) = res {
        eprintln!("Application error: {:#}", err);
    }

    Ok(())
}

fn detect_best_protocol(
    picker: &ratatui_image::picker::Picker,
) -> ratatui_image::picker::ProtocolType {
    // 1. In KDE Konsole, Sixel is the native, fully supported high-resolution protocol
    if is_konsole() {
        return ratatui_image::picker::ProtocolType::Sixel;
    }

    // 2. In native Kitty or Ghostty with Kitty graphics protocol support
    if is_native_kitty() {
        return ratatui_image::picker::ProtocolType::Kitty;
    }

    // 3. If query stdio detected Sixel support (Foot, WezTerm, etc.)
    if picker.protocol_type() == ratatui_image::picker::ProtocolType::Sixel {
        return ratatui_image::picker::ProtocolType::Sixel;
    }

    // 4. Default to Sixel on KDE Plasma sessions
    if std::env::var("KDE_FULL_SESSION").is_ok() {
        return ratatui_image::picker::ProtocolType::Sixel;
    }

    // Universal TrueColor halfblocks fallback
    ratatui_image::picker::ProtocolType::Halfblocks
}

pub fn is_konsole() -> bool {
    if std::env::var("KONSOLE_VERSION").is_ok()
        || std::env::var("KONSOLE_DBUS_SERVICE").is_ok()
        || std::env::var("KONSOLE_DBUS_SESSION").is_ok()
    {
        return true;
    }

    #[cfg(target_os = "linux")]
    {
        let mut pid = std::process::id();
        for _ in 0..6 {
            if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
                let parts: Vec<&str> = stat.split_whitespace().collect();
                if parts.len() > 3 {
                    if let Ok(comm) = std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
                        let name = comm.trim().to_lowercase();
                        if name.contains("konsole") {
                            return true;
                        }
                    }
                    if let Ok(ppid) = parts[3].parse::<u32>() {
                        if ppid <= 1 {
                            break;
                        }
                        pid = ppid;
                        continue;
                    }
                }
            }
            break;
        }
    }

    if std::env::var("KDE_FULL_SESSION").is_ok() {
        return true;
    }

    false
}

pub fn is_native_kitty() -> bool {
    std::env::var("KITTY_WINDOW_ID").is_ok()
        || std::env::var("TERM")
            .map(|t| t == "xterm-kitty")
            .unwrap_or(false)
        || std::env::var("GHOSTTY_RESOURCES_DIR").is_ok()
}
