use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Suno API Clerk Session Token (JWT Bearer token or full Cookie string)
    pub session_token: Option<String>,
    /// Download destination folder
    pub download_dir: PathBuf,
    /// Template for output filenames: supported variables {title}, {id}, {artist}, {style}
    pub file_template: String,
    /// Whether to embed cover art and lyrics into ID3 tags
    pub embed_id3: bool,
    /// Whether to save cover art as separate .jpg/.png file
    pub save_cover_image: bool,
    /// Whether to save lyrics/prompt as separate .txt file
    pub save_lyrics_file: bool,
    /// Max simultaneous downloads
    pub concurrency: usize,
}

impl Default for Config {
    fn default() -> Self {
        let download_dir = dirs::audio_dir()
            .map(|p| p.join("Suno"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|p| p.join("Music").join("Suno"))
                    .unwrap_or_else(|| PathBuf::from("./downloads"))
            });

        Self {
            session_token: None,
            download_dir,
            file_template: "{title} ({id})".to_string(),
            embed_id3: true,
            save_cover_image: false,
            save_lyrics_file: false,
            concurrency: 3,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join("suno-downloader").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("./config.toml"))
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<Config>(&content) {
                    return cfg;
                }
            }
        }
        // Fallback or default
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;
        Ok(())
    }

    pub fn sanitize_filename(name: &str) -> String {
        // Replace illegal filesystem characters: / \ : * ? " < > |
        let clean = name
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                '\0'..='\x1f' => '_',
                _ => c,
            })
            .collect::<String>();

        // Trim leading and trailing spaces and dots
        let trimmed = clean.trim().trim_matches('.').trim();
        if trimmed.is_empty() {
            "track".to_string()
        } else {
            trimmed.to_string()
        }
    }

    pub fn format_filename(&self, title: &str, id: &str, artist: &str, style: &str) -> String {
        let sanitized_title = Self::sanitize_filename(title);
        let sanitized_artist = Self::sanitize_filename(artist);
        let sanitized_style = Self::sanitize_filename(style);

        let mut res = self
            .file_template
            .replace("{title}", &sanitized_title)
            .replace("{id}", id)
            .replace("{artist}", &sanitized_artist)
            .replace("{style}", &sanitized_style);

        if res.is_empty() {
            res = format!("{} ({})", sanitized_title, id);
        }

        Self::sanitize_filename(&res)
    }
}
