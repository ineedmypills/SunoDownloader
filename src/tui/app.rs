use crate::api::SunoClient;
use crate::config::Config;
use crate::downloader::{DownloadProgressMsg, Downloader};
use crate::models::{Clip, DownloadStatus, TrackItem};
use crate::tui::icons::Icons;
use anyhow::Result;
use ratatui::style::Color;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveModal {
    None,
    AddUrl,
    Settings,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Token,
    AutoDetectBtn,
    DownloadDir,
    FileTemplate,
    Concurrency,
    GraphicsProtocol,
    EmbedId3,
    SaveCover,
    SaveLyrics,
    SaveBtn,
    CancelBtn,
}

impl SettingsField {
    pub fn next(self) -> Self {
        match self {
            SettingsField::Token => SettingsField::AutoDetectBtn,
            SettingsField::AutoDetectBtn => SettingsField::DownloadDir,
            SettingsField::DownloadDir => SettingsField::FileTemplate,
            SettingsField::FileTemplate => SettingsField::Concurrency,
            SettingsField::Concurrency => SettingsField::GraphicsProtocol,
            SettingsField::GraphicsProtocol => SettingsField::EmbedId3,
            SettingsField::EmbedId3 => SettingsField::SaveCover,
            SettingsField::SaveCover => SettingsField::SaveLyrics,
            SettingsField::SaveLyrics => SettingsField::SaveBtn,
            SettingsField::SaveBtn => SettingsField::CancelBtn,
            SettingsField::CancelBtn => SettingsField::Token,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SettingsField::Token => SettingsField::CancelBtn,
            SettingsField::AutoDetectBtn => SettingsField::Token,
            SettingsField::DownloadDir => SettingsField::AutoDetectBtn,
            SettingsField::FileTemplate => SettingsField::DownloadDir,
            SettingsField::Concurrency => SettingsField::FileTemplate,
            SettingsField::GraphicsProtocol => SettingsField::Concurrency,
            SettingsField::EmbedId3 => SettingsField::GraphicsProtocol,
            SettingsField::SaveCover => SettingsField::EmbedId3,
            SettingsField::SaveLyrics => SettingsField::SaveCover,
            SettingsField::SaveBtn => SettingsField::SaveLyrics,
            SettingsField::CancelBtn => SettingsField::SaveBtn,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Warning,
    Info,
    Error,
}

#[derive(Debug, Clone)]
pub struct SystemDiagnostic {
    pub level: DiagnosticLevel,
    pub title: &'static str,
    pub description: String,
    pub hint: &'static str,
}

pub struct StatusMessage {
    pub text: String,
    pub color: Color,
    pub expires_at: Instant,
}

pub struct App {
    pub config: Config,
    pub icons: Icons,
    pub items: Vec<TrackItem>,
    pub selected_index: usize,
    pub details_scroll: u16,

    pub search_query: String,
    pub search_mode: bool,

    pub active_modal: ActiveModal,
    pub url_input: String,

    // Settings modal editing
    pub settings_field: SettingsField,
    pub settings_token_input: String,
    pub settings_download_dir_input: String,
    pub settings_file_template_input: String,
    pub settings_concurrency: usize,
    pub settings_embed_id3: bool,
    pub settings_save_cover: bool,
    pub settings_save_lyrics: bool,

    pub status_msg: Option<StatusMessage>,
    pub is_loading_feed: bool,
    pub current_feed_page: usize,

    // Channels
    pub download_tx: UnboundedSender<DownloadProgressMsg>,
    pub download_rx: UnboundedReceiver<DownloadProgressMsg>,
    pub feed_tx: UnboundedSender<Result<Vec<Clip>>>,
    pub feed_rx: UnboundedReceiver<Result<Vec<Clip>>>,
    pub single_tx: UnboundedSender<Result<Clip>>,
    pub single_rx: UnboundedReceiver<Result<Clip>>,

    // Cover art image caching via ratatui-image
    pub picker: Option<Picker>,
    pub cover_cache: HashMap<String, StatefulProtocol>,
    pub raw_cover_cache: HashMap<String, Vec<u8>>,
    pub cover_tx: UnboundedSender<(String, Vec<u8>)>,
    pub cover_rx: UnboundedReceiver<(String, Vec<u8>)>,
    pub pending_covers: HashSet<String>,

    // In-TUI Audio Player
    pub player: crate::player::AudioPlayer,
    pub loading_audio_id: Option<String>,
    pub audio_ready_tx: UnboundedSender<(String, String, String)>,
    pub audio_ready_rx: UnboundedReceiver<(String, String, String)>,
    pub mouse_pos: Option<(u16, u16)>,
    pub anim_frame: usize,

    pub should_quit: bool,
}

impl App {
    pub fn new(mut config: Config, picker: Option<Picker>) -> Self {
        let (download_tx, download_rx) = unbounded_channel();
        let (feed_tx, feed_rx) = unbounded_channel();
        let (single_tx, single_rx) = unbounded_channel();
        let (cover_tx, cover_rx) = unbounded_channel();
        let (audio_ready_tx, audio_ready_rx) = unbounded_channel();

        let mut initial_status = None;
        if config
            .session_token
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            if let Some(token) = crate::browser::BrowserTokenDetector::detect() {
                config.session_token = Some(token);
                let _ = config.save();
                initial_status = Some(StatusMessage {
                    text: "✓ Suno session automatically detected & decrypted from browser!"
                        .to_string(),
                    color: Color::Green,
                    expires_at: Instant::now() + Duration::from_secs(6),
                });
            }
        }

        let token_val = config.session_token.clone().unwrap_or_default();
        let dl_dir = config.download_dir.to_string_lossy().to_string();
        let file_tpl = config.file_template.clone();
        let conc = config.concurrency;
        let id3 = config.embed_id3;
        let sc = config.save_cover_image;
        let sl = config.save_lyrics_file;

        Self {
            config,
            icons: Icons::detect(),
            items: Vec::new(),
            selected_index: 0,
            details_scroll: 0,
            search_query: String::new(),
            search_mode: false,
            active_modal: ActiveModal::None,
            url_input: String::new(),
            settings_field: SettingsField::Token,
            settings_token_input: token_val,
            settings_download_dir_input: dl_dir,
            settings_file_template_input: file_tpl,
            settings_concurrency: conc,
            settings_embed_id3: id3,
            settings_save_cover: sc,
            settings_save_lyrics: sl,
            status_msg: initial_status,
            is_loading_feed: false,
            current_feed_page: 1,
            download_tx,
            download_rx,
            feed_tx,
            feed_rx,
            single_tx,
            single_rx,
            picker,
            cover_cache: HashMap::new(),
            raw_cover_cache: HashMap::new(),
            cover_tx,
            cover_rx,
            pending_covers: HashSet::new(),
            player: crate::player::AudioPlayer::new(),
            loading_audio_id: None,
            audio_ready_tx,
            audio_ready_rx,
            mouse_pos: None,
            anim_frame: 0,
            should_quit: false,
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>, color: Color) {
        self.status_msg = Some(StatusMessage {
            text: text.into(),
            color,
            expires_at: Instant::now() + Duration::from_secs(5),
        });
    }

    pub fn clear_expired_status(&mut self) {
        self.anim_frame = self.anim_frame.wrapping_add(1);
        if let Some(msg) = &self.status_msg {
            if Instant::now() > msg.expires_at {
                self.status_msg = None;
            }
        }
    }

    pub fn system_diagnostics(&self) -> Vec<SystemDiagnostic> {
        let mut list = Vec::new();

        // 1. Audio player check
        if self.player.backend() == crate::player::PlayerBackend::None {
            list.push(SystemDiagnostic {
                level: DiagnosticLevel::Warning,
                title: "Audio Player Missing",
                description: "Neither 'mpv' nor 'ffplay' is installed in system PATH. In-TUI audio preview is disabled.".to_string(),
                hint: "Install mpv: sudo apt install mpv / pacman -S mpv / dnf install mpv / brew install mpv",
            });
        }

        // 2. Audio transcoder check
        if !crate::player::which("ffmpeg") {
            list.push(SystemDiagnostic {
                level: DiagnosticLevel::Info,
                title: "Transcoder Missing (ffmpeg)",
                description: "'ffmpeg' is not found. Audio streams will be saved as raw .m4a instead of transcoded 320 kbps MP3.".to_string(),
                hint: "Install ffmpeg: sudo apt install ffmpeg / pacman -S ffmpeg / dnf install ffmpeg",
            });
        }

        // 3. Authorization check
        let is_auth = self
            .config
            .session_token
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !is_auth {
            list.push(SystemDiagnostic {
                level: DiagnosticLevel::Info,
                title: "Public Mode (No Token)",
                description: "Suno session token is not configured. Personal library sync is disabled. Only public links can be fetched.".to_string(),
                hint: "Press [c] to set token or click 'Auto-Detect Token' from your browser.",
            });
        }

        list
    }

    pub fn primary_warning(&self) -> Option<SystemDiagnostic> {
        self.system_diagnostics()
            .into_iter()
            .find(|d| d.level == DiagnosticLevel::Warning || d.level == DiagnosticLevel::Error)
    }

    pub fn play_current_track(&mut self) {
        if self.player.backend() == crate::player::PlayerBackend::None {
            self.set_status(
                "⚠ Audio player missing! Install 'mpv' (sudo apt install mpv / pacman -S mpv) to enable audio preview.",
                Color::Rgb(251, 191, 36),
            );
            return;
        }

        let (id, title, mango_url, direct_url, local_path) = match self.current_item() {
            Some(item) => {
                let id = item.clip.id.clone();
                let title = item.clip.display_title();
                let mango_url = item.clip.get_mango_m4a_url().unwrap_or_else(|| {
                    format!("https://d2lwuy8qc234o3.cloudfront.net/1/clip/{}.m4a", id)
                });
                let direct_url = item.clip.get_audio_url();
                let local_path = match &item.download_status {
                    DownloadStatus::Completed { path } if path.exists() => {
                        Some(path.to_string_lossy().to_string())
                    }
                    _ => {
                        let default_path = self.config.download_dir.join(format!(
                            "{}.mp3",
                            self.config.format_filename(
                                &item.clip.display_title(),
                                &item.clip.id,
                                &item.clip.display_artist(),
                                &item.clip.display_tags()
                            )
                        ));
                        if default_path.exists() {
                            Some(default_path.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    }
                };
                (id, title, mango_url, direct_url, local_path)
            }
            None => return,
        };

        // If already playing this track, toggle pause
        if self.player.current_id() == Some(&id) && self.loading_audio_id.is_none() {
            self.player.toggle_pause();
            if self.player.is_paused() {
                self.set_status(format!("⏸ Paused: {}", title), Color::Yellow);
            } else {
                self.set_status(format!("▶ Resumed: {}", title), Color::Green);
            }
            return;
        }

        // Positive UI: Immediately stop any previous audio playback!
        self.player.stop();
        self.loading_audio_id = Some(id.clone());

        // 1. Check if downloaded locally first
        if let Some(path) = local_path {
            self.loading_audio_id = None;
            self.player.play(id, title.clone(), &path);
            self.set_status(format!("▶ Playing local: {}", title), Color::Green);
            return;
        }

        // 2. Check if cached decrypted audio exists in ~/.cache/suno-tui/audio/{id}.m4a
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("suno-tui/audio");
        let cache_file = cache_dir.join(format!("{}.m4a", id));
        if cache_file.exists()
            && std::fs::metadata(&cache_file)
                .map(|m| m.len() > 1024)
                .unwrap_or(false)
        {
            self.loading_audio_id = None;
            let p = cache_file.to_string_lossy().to_string();
            self.player.play(id, title.clone(), &p);
            self.set_status(format!("▶ Playing cached: {}", title), Color::Green);
            return;
        }

        // 3. Otherwise, fetch and decrypt via Mango DRM in background
        if let Some(token) = self.config.session_token.clone() {
            self.set_status(format!("⠋ Preparing stream: {}...", title), Color::Cyan);
            let id_clone = id.clone();
            let title_clone = title.clone();
            let audio_tx = self.audio_ready_tx.clone();

            tokio::spawn(async move {
                let _ = std::fs::create_dir_all(&cache_dir);
                if let Ok(decryptor) = crate::mango::MangoDecryptor::new(token) {
                    if let Ok((key, iv)) = decryptor.fetch_rights(&id_clone).await {
                        let client = reqwest::Client::new();
                        if let Ok(resp) = client.get(&mango_url).send().await {
                            if resp.status().is_success() {
                                if let Ok(mut bytes) = resp.bytes().await.map(|b| b.to_vec()) {
                                    if crate::mango::MangoDecryptor::decrypt_stream(
                                        &key, &iv, &mut bytes,
                                    )
                                    .is_ok()
                                        && std::fs::write(&cache_file, &bytes).is_ok()
                                    {
                                        let _ = audio_tx.send((
                                            id_clone,
                                            title_clone,
                                            cache_file.to_string_lossy().to_string(),
                                        ));
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }

                // Fallback to direct stream
                let fallback_url = format!("https://cdn1.suno.ai/{}.mp4", id_clone);
                let _ = audio_tx.send((id_clone, title_clone, fallback_url));
            });
        } else {
            self.loading_audio_id = None;
            self.player.play(id, title.clone(), &direct_url);
            self.set_status(format!("▶ Streaming online: {}", title), Color::Green);
        }
    }

    pub fn stop_playback(&mut self) {
        self.loading_audio_id = None;
        self.player.stop();
        self.set_status("⏹ Playback stopped", Color::DarkGray);
    }

    /// Returns list of visible items matching search query
    pub fn visible_indices(&self) -> Vec<usize> {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            (0..self.items.len()).collect()
        } else {
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    let title = item.clip.display_title().to_lowercase();
                    let artist = item.clip.display_artist().to_lowercase();
                    let tags = item.clip.display_tags().to_lowercase();
                    let prompt = item.clip.display_prompt().to_lowercase();
                    let id = item.clip.id.to_lowercase();
                    title.contains(&query)
                        || artist.contains(&query)
                        || tags.contains(&query)
                        || prompt.contains(&query)
                        || id.contains(&query)
                })
                .map(|(i, _)| i)
                .collect()
        }
    }

    pub fn current_item(&self) -> Option<&TrackItem> {
        let vis = self.visible_indices();
        if vis.is_empty() {
            None
        } else {
            let actual_idx = vis.get(self.selected_index.min(vis.len().saturating_sub(1)))?;
            self.items.get(*actual_idx)
        }
    }

    pub fn ensure_current_cover(&mut self) {
        if let Some(item) = self.current_item() {
            let id = item.clip.id.clone();
            if !self.cover_cache.contains_key(&id) && !self.pending_covers.contains(&id) {
                if let Some(url) = item
                    .clip
                    .image_large_url
                    .clone()
                    .or_else(|| item.clip.image_url.clone())
                {
                    self.pending_covers.insert(id.clone());
                    let tx = self.cover_tx.clone();
                    let token = self.config.session_token.clone();
                    tokio::spawn(async move {
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
                            .build()
                            .unwrap_or_default();
                        let mut req = client.get(&url);
                        if let Some(tok) = token {
                            let clean_jwt = crate::api::SunoClient::extract_token(&tok);
                            req = req.header(
                                reqwest::header::AUTHORIZATION,
                                format!("Bearer {}", clean_jwt),
                            );
                        }
                        if let Ok(res) = req.send().await {
                            if res.status().is_success() {
                                if let Ok(bytes) = res.bytes().await {
                                    let _ = tx.send((id, bytes.to_vec()));
                                    return;
                                }
                            }
                        }
                        let _ = tx.send((id, Vec::new()));
                    });
                }
            }
        }
    }

    pub fn move_cursor_down(&mut self) {
        let count = self.visible_indices().len();
        if count > 0 && self.selected_index + 1 < count {
            self.selected_index += 1;
            self.details_scroll = 0;
            self.ensure_current_cover();
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.details_scroll = 0;
            self.ensure_current_cover();
        }
    }

    pub fn move_cursor_first(&mut self) {
        self.selected_index = 0;
        self.details_scroll = 0;
        self.ensure_current_cover();
    }

    pub fn move_cursor_last(&mut self) {
        let count = self.visible_indices().len();
        if count > 0 {
            self.selected_index = count - 1;
            self.details_scroll = 0;
            self.ensure_current_cover();
        }
    }

    pub fn page_down(&mut self, page_size: usize) {
        let count = self.visible_indices().len();
        if count > 0 {
            self.selected_index = (self.selected_index + page_size).min(count - 1);
            self.details_scroll = 0;
            self.ensure_current_cover();
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected_index = self.selected_index.saturating_sub(page_size);
        self.details_scroll = 0;
        self.ensure_current_cover();
    }

    pub fn scroll_details_down(&mut self) {
        self.details_scroll = self.details_scroll.saturating_add(2);
    }

    pub fn scroll_details_up(&mut self) {
        self.details_scroll = self.details_scroll.saturating_sub(2);
    }

    pub fn toggle_select_current(&mut self) {
        let vis = self.visible_indices();
        if let Some(idx) = vis.get(self.selected_index) {
            if let Some(item) = self.items.get_mut(*idx) {
                item.selected = !item.selected;
            }
        }
    }

    pub fn select_all_visible(&mut self) {
        let vis = self.visible_indices();
        for idx in vis {
            if let Some(item) = self.items.get_mut(idx) {
                item.selected = true;
            }
        }
    }

    pub fn deselect_all(&mut self) {
        for item in &mut self.items {
            item.selected = false;
        }
    }

    pub fn add_or_update_clips(&mut self, clips: Vec<Clip>) {
        let mut existing_ids: HashSet<String> =
            self.items.iter().map(|it| it.clip.id.clone()).collect();
        let mut added_count = 0;

        for clip in clips {
            if !existing_ids.contains(&clip.id) {
                existing_ids.insert(clip.id.clone());
                self.items.push(TrackItem::new(clip));
                added_count += 1;
            } else if let Some(existing) = self.items.iter_mut().find(|it| it.clip.id == clip.id) {
                existing.clip = clip;
            }
        }

        if added_count > 0 {
            self.set_status(
                format!("Added {} track(s) to library", added_count),
                Color::Green,
            );
            self.ensure_current_cover();
        }
    }

    /// Refresh library from Suno API
    pub fn refresh_library(&mut self) {
        if self.is_loading_feed {
            return;
        }
        if self
            .config
            .session_token
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            self.set_status(
                "No session token set! Press [c] to configure your Suno auth token.",
                Color::Yellow,
            );
            return;
        }

        self.is_loading_feed = true;
        self.set_status("Fetching personal library from Suno...", Color::Cyan);

        let token = self.config.session_token.clone();
        let page = self.current_feed_page;
        let tx = self.feed_tx.clone();

        tokio::spawn(async move {
            let client = SunoClient::new(token);
            match client {
                Ok(c) => {
                    let res = c.fetch_feed_page(page).await;
                    let _ = tx.send(res);
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
        });
    }

    /// Fetch a single clip by URL or UUID
    pub fn fetch_single_clip(&mut self, url_or_uuid: String) {
        let trimmed = url_or_uuid.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        self.set_status(format!("Looking up track: {}...", trimmed), Color::Cyan);
        let token = self.config.session_token.clone();
        let tx = self.single_tx.clone();

        tokio::spawn(async move {
            let client = SunoClient::new(token);
            match client {
                Ok(c) => {
                    let res = c.fetch_clip(&trimmed).await;
                    let _ = tx.send(res);
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
        });
    }

    pub fn open_settings(&mut self) {
        self.settings_field = SettingsField::Token;
        self.settings_token_input = self.config.session_token.clone().unwrap_or_default();
        self.settings_download_dir_input = self.config.download_dir.to_string_lossy().to_string();
        self.settings_file_template_input = self.config.file_template.clone();
        self.settings_concurrency = self.config.concurrency;
        self.settings_embed_id3 = self.config.embed_id3;
        self.settings_save_cover = self.config.save_cover_image;
        self.settings_save_lyrics = self.config.save_lyrics_file;
        self.active_modal = ActiveModal::Settings;
    }

    pub fn save_settings(&mut self) {
        let new_token = self.settings_token_input.trim().to_string();
        let token_changed = self.config.session_token.as_deref()
            != if new_token.is_empty() {
                None
            } else {
                Some(new_token.as_str())
            };

        self.config.session_token = if new_token.is_empty() {
            None
        } else {
            Some(new_token)
        };

        let new_dir = self.settings_download_dir_input.trim();
        if !new_dir.is_empty() {
            self.config.download_dir = std::path::PathBuf::from(new_dir);
        }

        let new_tpl = self.settings_file_template_input.trim();
        if !new_tpl.is_empty() {
            self.config.file_template = new_tpl.to_string();
        }

        self.config.concurrency = self.settings_concurrency.clamp(1, 10);
        self.config.embed_id3 = self.settings_embed_id3;
        self.config.save_cover_image = self.settings_save_cover;
        self.config.save_lyrics_file = self.settings_save_lyrics;

        if let Err(e) = self.config.save() {
            self.set_status(format!("Failed to save config: {:#}", e), Color::Red);
        } else {
            self.set_status(
                "✓ Configuration saved to ~/.config/suno-downloader/config.toml",
                Color::Green,
            );
        }

        if token_changed && self.config.session_token.is_some() {
            self.refresh_library();
        }

        self.active_modal = ActiveModal::None;
    }

    /// Auto-detect session token from browser cookie stores (Chrome, Chrome-Unstable, Brave, Firefox)
    pub fn try_autodetect_browser_token(&mut self) {
        self.set_status(
            "Scanning Chrome, Chrome Unstable, Brave, Firefox & KWallet...",
            Color::Cyan,
        );

        if let Some(token) = crate::browser::BrowserTokenDetector::detect() {
            self.settings_token_input = token.clone();
            self.config.session_token = Some(token);
            let _ = self.config.save();
            self.set_status(
                "✓ Suno session token successfully extracted & decrypted from browser!",
                Color::Green,
            );
            self.refresh_library();
        } else {
            self.set_status(
                "Browser token not found or locked. Please paste token manually.",
                Color::Yellow,
            );
        }
    }

    /// Start downloading all selected tracks (or current if none selected)
    pub fn start_downloads(&mut self) {
        let target_indices: Vec<usize> = {
            let selected: Vec<usize> = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, it)| {
                    it.selected && !matches!(it.download_status, DownloadStatus::Downloading { .. })
                })
                .map(|(i, _)| i)
                .collect();

            if !selected.is_empty() {
                selected
            } else if let Some(item) = self.current_item() {
                if !matches!(item.download_status, DownloadStatus::Downloading { .. }) {
                    let vis = self.visible_indices();
                    let current_actual = vis[self.selected_index];
                    vec![current_actual]
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        };

        if target_indices.is_empty() {
            self.set_status(
                "No tracks to download (all selected tracks are already downloading)",
                Color::Yellow,
            );
            return;
        }

        let count = target_indices.len();
        self.set_status(
            format!("Queued {} track(s) for download", count),
            Color::Cyan,
        );

        for &idx in &target_indices {
            self.items[idx].download_status = DownloadStatus::Queued;
        }

        let clips_to_download: Vec<Clip> = target_indices
            .iter()
            .map(|&idx| self.items[idx].clip.clone())
            .collect();

        let downloader = Downloader::new(self.config.clone());
        let tx = self.download_tx.clone();
        let concurrency = self.config.concurrency.max(1);
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
        let dl = std::sync::Arc::new(downloader);

        tokio::spawn(async move {
            for clip in clips_to_download {
                if let Ok(permit) = sem.clone().acquire_owned().await {
                    let dl = dl.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(e) = dl.download_clip(&clip, Some(tx.clone())).await {
                            let _ = tx.send(DownloadProgressMsg::Failed {
                                clip_id: clip.id.clone(),
                                error: format!("{:#}", e),
                            });
                        }
                    });
                }
            }
        });
    }

    /// Process updates from background async channels
    pub fn update_channels(&mut self) {
        // Feed updates
        while let Ok(res) = self.feed_rx.try_recv() {
            self.is_loading_feed = false;
            match res {
                Ok(clips) => {
                    self.add_or_update_clips(clips);
                }
                Err(e) => {
                    self.set_status(format!("Feed error: {:#}", e), Color::Red);
                }
            }
        }

        // Single clip update
        while let Ok(res) = self.single_rx.try_recv() {
            match res {
                Ok(clip) => {
                    let title = clip.display_title();
                    let id = clip.id.clone();
                    self.add_or_update_clips(vec![clip]);
                    if let Some(pos) = self.items.iter().position(|it| it.clip.id == id) {
                        if let Some(vis_pos) =
                            self.visible_indices().iter().position(|&idx| idx == pos)
                        {
                            self.selected_index = vis_pos;
                        }
                    }
                    self.set_status(format!("Added track: {}", title), Color::Green);
                    self.ensure_current_cover();
                }
                Err(e) => {
                    self.set_status(format!("Fetch error: {:#}", e), Color::Red);
                }
            }
        }

        // Audio playback stream ready updates
        while let Ok((id, title, path)) = self.audio_ready_rx.try_recv() {
            if self.loading_audio_id.as_deref() == Some(&id) {
                self.loading_audio_id = None;
                self.player.play(id, title.clone(), &path);
                self.set_status(format!("▶ Playing: {}", title), Color::Green);
            }
        }

        // Cover image download updates
        while let Ok((id, bytes)) = self.cover_rx.try_recv() {
            self.pending_covers.remove(&id);
            if !bytes.is_empty() {
                self.raw_cover_cache.insert(id.clone(), bytes.clone());
                if let Ok(img) = image::load_from_memory(&bytes) {
                    if let Some(picker) = &mut self.picker {
                        let proto = picker.new_resize_protocol(img);
                        self.cover_cache.insert(id, proto);
                    }
                }
            }
        }

        // Download progress updates
        while let Ok(msg) = self.download_rx.try_recv() {
            match msg {
                DownloadProgressMsg::Started { clip_id } => {
                    if let Some(item) = self.items.iter_mut().find(|it| it.clip.id == clip_id) {
                        item.download_status = DownloadStatus::Downloading {
                            progress: 0.0,
                            downloaded_bytes: 0,
                            total_bytes: 0,
                        };
                    }
                }
                DownloadProgressMsg::Progress {
                    clip_id,
                    progress,
                    downloaded,
                    total,
                } => {
                    if let Some(item) = self.items.iter_mut().find(|it| it.clip.id == clip_id) {
                        item.download_status = DownloadStatus::Downloading {
                            progress,
                            downloaded_bytes: downloaded,
                            total_bytes: total,
                        };
                    }
                }
                DownloadProgressMsg::Completed { clip_id, file_path } => {
                    if let Some(item) = self.items.iter_mut().find(|it| it.clip.id == clip_id) {
                        item.download_status = DownloadStatus::Completed {
                            path: file_path.clone(),
                        };
                        item.selected = false;
                    }
                    self.set_status(
                        format!(
                            "Downloaded: {}",
                            file_path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        Color::Green,
                    );
                }
                DownloadProgressMsg::Failed { clip_id, error } => {
                    if let Some(item) = self.items.iter_mut().find(|it| it.clip.id == clip_id) {
                        item.download_status = DownloadStatus::Failed {
                            error: error.clone(),
                        };
                    }
                    self.set_status(format!("Download failed: {}", error), Color::Red);
                }
            }
        }
    }

    pub fn image_protocol_label(&self) -> &'static str {
        if let Some(picker) = &self.picker {
            match picker.protocol_type() {
                ratatui_image::picker::ProtocolType::Sixel => "Sixel",
                ratatui_image::picker::ProtocolType::Kitty => "Kitty",
                ratatui_image::picker::ProtocolType::Iterm2 => "iTerm2",
                ratatui_image::picker::ProtocolType::Halfblocks => "Halfblocks",
            }
        } else {
            "Disabled"
        }
    }

    pub fn cycle_image_protocol(&mut self) {
        let Some(picker) = &mut self.picker else {
            self.set_status(
                "Cover art graphics renderer not available in this terminal",
                Color::Yellow,
            );
            return;
        };

        let current = picker.protocol_type();
        let next = match current {
            ratatui_image::picker::ProtocolType::Sixel => {
                ratatui_image::picker::ProtocolType::Halfblocks
            }
            ratatui_image::picker::ProtocolType::Halfblocks => {
                if crate::is_native_kitty() {
                    ratatui_image::picker::ProtocolType::Kitty
                } else {
                    ratatui_image::picker::ProtocolType::Sixel
                }
            }
            ratatui_image::picker::ProtocolType::Kitty => {
                ratatui_image::picker::ProtocolType::Halfblocks
            }
            ratatui_image::picker::ProtocolType::Iterm2 => {
                ratatui_image::picker::ProtocolType::Halfblocks
            }
        };

        picker.set_protocol_type(next);

        // Rebuild cover_cache for all cached images with the new protocol
        self.cover_cache.clear();
        for (id, bytes) in &self.raw_cover_cache {
            if !bytes.is_empty() {
                if let Ok(img) = image::load_from_memory(bytes) {
                    let proto = picker.new_resize_protocol(img);
                    self.cover_cache.insert(id.clone(), proto);
                }
            }
        }

        let label = self.image_protocol_label();
        self.set_status(
            format!("🖼 Graphics protocol switched to: {}", label),
            Color::Cyan,
        );
    }
}
