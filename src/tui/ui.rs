use crate::models::DownloadStatus;
use crate::player::PlayerBackend;
use crate::tui::app::{ActiveModal, App, SettingsField};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, Copy)]
pub struct FooterHitboxes {
    pub quit_rect: Rect,
    pub add_rect: Rect,
    pub search_rect: Rect,
    pub refresh_rect: Rect,
    pub settings_rect: Rect,
    pub help_rect: Rect,
}

pub fn get_footer_hitboxes(area: Rect) -> FooterHitboxes {
    let row = area.y + 1;

    // Left: [q] Quit
    let quit_rect = Rect {
        x: area.x + 2,
        y: row,
        width: 8,
        height: 1,
    };

    // Center: [a] Add Track • [/] Search • [r] Sync
    let center_total_w = 37u16;
    let center_start = area.x + (area.width.saturating_sub(center_total_w)) / 2;

    let add_rect = Rect {
        x: center_start,
        y: row,
        width: 13,
        height: 1,
    };
    let search_rect = Rect {
        x: center_start + 13 + 3,
        y: row,
        width: 10,
        height: 1,
    };
    let refresh_rect = Rect {
        x: center_start + 13 + 3 + 10 + 3,
        y: row,
        width: 8,
        height: 1,
    };

    // Right: [c] Settings • [?] Help
    let right_total_w = 23u16;
    let right_start = area.x + area.width.saturating_sub(right_total_w + 2);

    let settings_rect = Rect {
        x: right_start,
        y: row,
        width: 12,
        height: 1,
    };
    let help_rect = Rect {
        x: right_start + 12 + 3,
        y: row,
        width: 8,
        height: 1,
    };

    FooterHitboxes {
        quit_rect,
        add_rect,
        search_rect,
        refresh_rect,
        settings_rect,
        help_rect,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InspectorButtonRects {
    pub art_rect: Rect,
    pub play_rect: Rect,
    pub stop_rect: Rect,
    pub download_rect: Rect,
    pub select_rect: Rect,
}

pub fn get_inspector_button_rects(details_area: Rect) -> InspectorButtonRects {
    let meta_x = details_area.x + 24;
    let meta_y = details_area.y + 1;

    InspectorButtonRects {
        art_rect: Rect {
            x: details_area.x + 1,
            y: details_area.y + 1,
            width: 20,
            height: 11,
        },
        play_rect: Rect {
            x: meta_x + 12,
            y: meta_y + 5,
            width: 17,
            height: 1,
        },
        stop_rect: Rect {
            x: meta_x + 30,
            y: meta_y + 5,
            width: 16,
            height: 1,
        },
        download_rect: Rect {
            x: meta_x + 12,
            y: meta_y + 6,
            width: 20,
            height: 1,
        },
        select_rect: Rect {
            x: meta_x + 33,
            y: meta_y + 6,
            width: 24,
            height: 1,
        },
    }
}

pub fn settings_modal_rect(r: Rect) -> Rect {
    let w = (r.width * 75 / 100).clamp(66, 88).min(r.width);
    let h = 24u16.min(r.height.saturating_sub(2));
    let x = (r.width.saturating_sub(w)) / 2;
    let y = (r.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

pub fn settings_inner_chunks(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 0: Top Subtitle / navigation tips
            Constraint::Length(3), // 1: Token input block
            Constraint::Length(1), // 2: Auto-detect button
            Constraint::Length(3), // 3: Download Directory input block
            Constraint::Length(3), // 4: Filename Template input block
            Constraint::Length(1), // 5: Concurrency selector
            Constraint::Length(1), // 6: Graphics protocol selector
            Constraint::Length(3), // 7: Checkboxes (ID3, Cover, Lyrics)
            Constraint::Length(1), // 8: System Diagnostics status line
            Constraint::Length(2), // 9: Action Buttons (Save, Cancel)
        ])
        .margin(1)
        .split(area)
        .to_vec()
}

pub fn add_url_modal_rect(r: Rect) -> Rect {
    let w = (r.width * 60 / 100).clamp(50, 75).min(r.width);
    let h = 10u16.min(r.height.saturating_sub(2));
    let x = (r.width.saturating_sub(w)) / 2;
    let y = (r.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

pub fn help_modal_rect(r: Rect) -> Rect {
    let w = (r.width * 72 / 100).clamp(58, 88).min(r.width);
    let h = 29u16.min(r.height.saturating_sub(2));
    let x = (r.width.saturating_sub(w)) / 2;
    let y = (r.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

pub fn is_hovered(mouse_pos: Option<(u16, u16)>, rect: Rect) -> bool {
    if let Some((x, y)) = mouse_pos {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    } else {
        false
    }
}

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    let warning = app.primary_warning();
    let has_warning = warning.is_some() && size.height > 18;

    let is_modal_active = app.active_modal != ActiveModal::None;

    if let (true, Some(warn)) = (has_warning, &warning) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // System Notice / Attention banner
                Constraint::Min(8),    // Main content (two panels)
                Constraint::Length(3), // Footer / Action buttons
            ])
            .split(size);

        render_header(f, app, chunks[0], is_modal_active);
        render_warning_banner(f, app, chunks[1], warn, is_modal_active);
        render_body(f, app, chunks[2], is_modal_active);
        render_footer(f, app, chunks[3], is_modal_active);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Main content (two panels)
                Constraint::Length(3), // Footer / Action buttons
            ])
            .split(size);

        render_header(f, app, chunks[0], is_modal_active);
        render_body(f, app, chunks[1], is_modal_active);
        render_footer(f, app, chunks[2], is_modal_active);
    }

    // If modal is active, dim backdrop and render modal on top
    match app.active_modal {
        ActiveModal::AddUrl => render_add_url_modal(f, app),
        ActiveModal::Settings => render_settings_modal(f, app),
        ActiveModal::Help => render_help_modal(f, app),
        ActiveModal::None => {}
    }
}

fn render_warning_banner(
    f: &mut Frame,
    app: &App,
    area: Rect,
    warning: &crate::tui::app::SystemDiagnostic,
    dimmed: bool,
) {
    let is_banner_hovered = is_hovered(app.mouse_pos, area);
    let border_color = if dimmed {
        Color::DarkGray
    } else if is_banner_hovered {
        Color::Rgb(254, 240, 138)
    } else {
        match warning.level {
            crate::tui::app::DiagnosticLevel::Error => Color::Rgb(248, 113, 113),
            crate::tui::app::DiagnosticLevel::Warning => Color::Rgb(251, 191, 36),
            crate::tui::app::DiagnosticLevel::Info => Color::Rgb(56, 189, 248),
        }
    };

    let title_span = Span::styled(
        format!(" ⚠ System Notice: {} (Click for Help) ", warning.title),
        Style::default()
            .fg(border_color)
            .add_modifier(Modifier::BOLD),
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title_span);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content_line = Line::from(vec![
        Span::styled(
            format!(" {} ", warning.description),
            Style::default().fg(if dimmed {
                Color::DarkGray
            } else {
                Color::White
            }),
        ),
        Span::styled(
            format!("• {}", warning.hint),
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    border_color
                })
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let p = Paragraph::new(content_line).alignment(Alignment::Left);
    f.render_widget(p, inner);
}

fn render_header(f: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let is_auth = app
        .config
        .session_token
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let auth_badge = if is_auth {
        Span::styled(
            format!(" {}Authorized ", app.icons.shield_auth),
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    Color::Rgb(74, 222, 128)
                })
                .bg(Color::Rgb(18, 38, 26))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" {}Public Only ", app.icons.shield_anon),
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    Color::Rgb(251, 191, 36)
                })
                .bg(Color::Rgb(40, 32, 18))
                .add_modifier(Modifier::BOLD),
        )
    };

    let selected_count = app.items.iter().filter(|i| i.selected).count();
    let total_count = app.items.len();
    let downloading_count = app
        .items
        .iter()
        .filter(|i| matches!(i.download_status, DownloadStatus::Downloading { .. }))
        .count();

    let mut stats_spans = vec![
        Span::raw(" Total: "),
        Span::styled(
            total_count.to_string(),
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::raw("Selected: "),
        Span::styled(
            selected_count.to_string(),
            if selected_count > 0 && !dimmed {
                Style::default()
                    .fg(Color::Rgb(250, 204, 21))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ];

    if downloading_count > 0 {
        stats_spans.push(Span::raw(" | "));
        stats_spans.push(Span::styled(
            format!("{}Active: {} ", app.icons.arrow_down, downloading_count),
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    Color::Rgb(56, 189, 248)
                })
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Now Playing status in header
    let is_player_active = app.player.is_active();
    let mut header_spans = vec![
        Span::styled(
            format!(" {}Suno.ai Downloader ", app.icons.music),
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    Color::Rgb(56, 189, 248)
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
        auth_badge,
        Span::styled(" │", Style::default().fg(Color::DarkGray)),
    ];

    if is_player_active {
        let eq_frames = [
            " ♫ ▂ ",
            " ♫ ▃ ",
            " ♫ ▅ ",
            " ♫ ▆ ",
            " ♫ ▇ ",
            " ♫ ▆ ",
            " ♫ ▅ ",
            " ♫ ▃ ",
        ];
        let eq_str = if app.player.is_paused() {
            " ⏸ PAUSED "
        } else {
            eq_frames[app.anim_frame % eq_frames.len()]
        };

        let track_name = app.player.current_title().unwrap_or("Track");
        header_spans.push(Span::raw(" "));
        header_spans.push(Span::styled(
            format!(" {} {} ", eq_str, track_name),
            Style::default()
                .fg(if app.player.is_paused() {
                    Color::Yellow
                } else {
                    Color::Green
                })
                .bg(Color::Rgb(18, 30, 22))
                .add_modifier(Modifier::BOLD),
        ));
    }

    let block_border_color = if dimmed {
        Color::Rgb(40, 50, 65)
    } else {
        Color::Rgb(70, 110, 160)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(block_border_color));

    let left_paragraph = Paragraph::new(Line::from(header_spans)).block(block);
    f.render_widget(left_paragraph, area);

    // Right-aligned stats
    let right_p = Paragraph::new(Line::from(stats_spans)).alignment(Alignment::Right);
    let right_rect = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: 1,
    };
    f.render_widget(right_p, right_rect);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(48), // Left panel: Track list
            Constraint::Percentage(52), // Right panel: Track details, cover art & lyrics
        ])
        .split(area);

    render_track_list(f, app, main_chunks[0], dimmed);
    render_track_details(f, app, main_chunks[1], dimmed);
}

fn render_track_list(f: &mut Frame, app: &App, area: Rect, dimmed: bool) {
    let search_height = if app.search_mode || !app.search_query.is_empty() {
        3
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(search_height), Constraint::Min(5)])
        .split(area);

    if search_height > 0 {
        let is_search_hovered = is_hovered(app.mouse_pos, chunks[0]);
        let search_border_color = if app.search_mode {
            Color::Yellow
        } else if is_search_hovered {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(search_border_color))
            .title(Span::styled(
                format!(" {}Search Filter [/] (Esc to clear) ", app.icons.search),
                Style::default().fg(search_border_color),
            ));

        let query_text = Line::from(vec![
            Span::styled(
                format!(" {} ", app.icons.search),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                &app.search_query,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            if app.search_mode {
                Span::styled("▍", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]);

        let p = Paragraph::new(query_text).block(search_block);
        f.render_widget(p, chunks[0]);
    }

    let vis_indices = app.visible_indices();
    let list_area = if search_height > 0 { chunks[1] } else { area };

    let is_list_hovered = is_hovered(app.mouse_pos, list_area);

    let title_text = if app.is_loading_feed {
        format!(" {}Track Library (Syncing with Suno...) ", app.icons.lyrics)
    } else {
        format!(
            " {}Track Library ({}/{}) [Click/Space: Select, p: Play] ",
            app.icons.lyrics,
            if vis_indices.is_empty() {
                0
            } else {
                app.selected_index + 1
            },
            vis_indices.len()
        )
    };

    let border_color = if dimmed {
        Color::Rgb(40, 50, 65)
    } else if is_list_hovered {
        Color::Rgb(100, 150, 210)
    } else {
        Color::Rgb(70, 110, 160)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title_text,
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD),
        ));

    if vis_indices.is_empty() {
        if app.items.is_empty() {
            let is_auth = app
                .config
                .session_token
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            let mpv_ready = app.player.backend() != PlayerBackend::None;
            let ffmpeg_ready = crate::player::which("ffmpeg");

            let mut lines = vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("  {}Suno Library Overview", app.icons.music),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            if is_auth {
                lines.push(Line::from(vec![
                    Span::styled("  • Account Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}Authorized", app.icons.shield_auth),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " (Press [r] to sync library from Suno)",
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  • Account Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}Public / Anonymous Mode", app.icons.shield_anon),
                        Style::default()
                            .fg(Color::Rgb(251, 191, 36))
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![Span::styled(
                    "    Press [c] to auto-detect token from Chrome/Firefox or enter manually.",
                    Style::default().fg(Color::DarkGray),
                )]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  System Diagnostics & Dependencies:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]));

            if mpv_ready {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {}Audio Player: ", app.icons.check),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(
                            "{} ready (in-TUI audio preview active)",
                            app.player.backend().name()
                        ),
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {}Audio Player: ", app.icons.cross),
                        Style::default()
                            .fg(Color::Rgb(251, 191, 36))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "Not installed (install 'mpv' to listen inside TUI)",
                        Style::default().fg(Color::Rgb(251, 191, 36)),
                    ),
                ]));
            }

            if ffmpeg_ready {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {}Transcoder:   ", app.icons.check),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "ffmpeg ready (pristine 320 kbps MP3 conversion active)",
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {}Transcoder:   ", app.icons.dot),
                        Style::default().fg(Color::Rgb(56, 189, 248)),
                    ),
                    Span::styled(
                        "ffmpeg not found (downloads will save as original .m4a)",
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  Quick Start:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(
                "    • Press [a] or click '+ Add Track' to fetch any public track URL",
            ));
            lines.push(Line::from(
                "    • Press [c] or click 'Settings' to configure token & directories",
            ));
            lines.push(Line::from(
                "    • Press [h] or click 'Help' for full controls & keybindings",
            ));

            let empty_p = Paragraph::new(lines)
                .block(block)
                .alignment(Alignment::Left);
            f.render_widget(empty_p, list_area);
            return;
        } else {
            let msg = "\n  No tracks match your search filter.\n\n  Press [/] to modify search or Esc to clear.";
            let empty_p = Paragraph::new(msg)
                .block(block)
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_p, list_area);
            return;
        }
    }

    let inner_height = list_area.height.saturating_sub(2) as usize;
    let selected_pos = app.selected_index.min(vis_indices.len().saturating_sub(1));

    let start_idx = if selected_pos >= inner_height {
        selected_pos.saturating_sub(inner_height / 2)
    } else {
        0
    };
    let end_idx = (start_idx + inner_height).min(vis_indices.len());

    let mut list_items = Vec::new();
    for (vis_pos, &actual_idx) in vis_indices[start_idx..end_idx].iter().enumerate() {
        let abs_pos = start_idx + vis_pos;
        let is_selected_cursor = abs_pos == selected_pos;
        let item = &app.items[actual_idx];

        // Check if mouse is hovering over this exact row
        let row_screen_y = list_area.y + 1 + vis_pos as u16;
        let is_row_hovered = app
            .mouse_pos
            .map(|(mx, my)| {
                my == row_screen_y && mx >= list_area.x && mx < list_area.x + list_area.width
            })
            .unwrap_or(false);

        // Checkbox indicator
        let checkbox = if item.selected {
            Span::styled(
                app.icons.check_box_checked,
                Style::default()
                    .fg(Color::Rgb(250, 204, 21))
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                app.icons.check_box_unchecked,
                Style::default().fg(Color::Rgb(100, 110, 125)),
            )
        };

        // Status indicator
        let status_span = match &item.download_status {
            DownloadStatus::Idle => {
                Span::styled(app.icons.dot, Style::default().fg(Color::DarkGray))
            }
            DownloadStatus::Queued => Span::styled(
                app.icons.hourglass,
                Style::default().fg(Color::Rgb(251, 191, 36)),
            ),
            DownloadStatus::Downloading { progress, .. } => {
                let pct = (progress * 100.0) as u32;
                Span::styled(
                    format!("{}{:2}% ", app.icons.arrow_down, pct),
                    Style::default()
                        .fg(Color::Rgb(56, 189, 248))
                        .add_modifier(Modifier::BOLD),
                )
            }
            DownloadStatus::Completed { .. } => Span::styled(
                app.icons.check,
                Style::default()
                    .fg(Color::Rgb(74, 222, 128))
                    .add_modifier(Modifier::BOLD),
            ),
            DownloadStatus::Failed { .. } => Span::styled(
                app.icons.cross,
                Style::default()
                    .fg(Color::Rgb(248, 113, 113))
                    .add_modifier(Modifier::BOLD),
            ),
        };

        let title_str = item.clip.display_title();
        let duration_str = item.clip.duration_formatted();

        let inner_width = (list_area.width as usize).saturating_sub(3);
        let fixed_width = 2 + 4 + 2 + 5 + 9; // cursor(2) + checkbox(4) + badge(2) + status(5) + duration(9) = 22
        let max_title_len = inner_width.saturating_sub(fixed_width);
        let safe_title = if title_str.chars().count() > max_title_len && max_title_len > 3 {
            format!(
                "{}...",
                title_str
                    .chars()
                    .take(max_title_len - 3)
                    .collect::<String>()
            )
        } else if max_title_len <= 3 {
            String::new()
        } else {
            title_str.to_string()
        };

        let is_track_playing = app.player.current_id() == Some(&item.clip.id);
        let is_track_loading = app.loading_audio_id.as_deref() == Some(&item.clip.id);
        let playing_badge = if is_track_loading {
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner = spinner_frames[app.anim_frame % spinner_frames.len()];
            Span::styled(
                format!("{} ", spinner),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_track_playing {
            if app.player.is_paused() {
                Span::styled(
                    "⏸ ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "♫ ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            }
        } else {
            Span::raw("")
        };

        let cursor_span = if is_selected_cursor {
            Span::styled(
                app.icons.cursor,
                Style::default()
                    .fg(Color::Rgb(250, 204, 21))
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("  ")
        };

        let title_style = if is_selected_cursor {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_track_playing {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else if is_row_hovered {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(210, 215, 225))
        };

        let line_spans = vec![
            cursor_span,
            checkbox,
            playing_badge,
            status_span,
            Span::styled(safe_title, title_style),
            Span::styled(
                format!("  [{}]", duration_str),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        let item_style = if is_selected_cursor {
            Style::default().bg(Color::Rgb(28, 44, 68))
        } else if is_row_hovered {
            Style::default().bg(Color::Rgb(22, 32, 48))
        } else {
            Style::default()
        };

        list_items.push(ListItem::new(Line::from(line_spans)).style(item_style));
    }

    let list = List::new(list_items).block(block);
    f.render_widget(list, list_area);
}

fn render_track_details(f: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let is_panel_hovered = is_hovered(app.mouse_pos, area);

    let border_color = if dimmed {
        Color::Rgb(40, 50, 65)
    } else if is_panel_hovered {
        Color::Rgb(160, 120, 210)
    } else {
        Color::Rgb(120, 90, 160)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {}Track Inspector & Lyrics ", app.icons.music),
            Style::default()
                .fg(if dimmed {
                    Color::DarkGray
                } else {
                    Color::White
                })
                .add_modifier(Modifier::BOLD),
        ));

    let item = match app.current_item() {
        Some(it) => it.clone(),
        None => {
            let empty_p =
                Paragraph::new("\n  Select a track on the left to view details and lyrics.")
                    .block(block)
                    .alignment(Alignment::Left)
                    .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty_p, area);
            return;
        }
    };

    let is_downloading = matches!(item.download_status, DownloadStatus::Downloading { .. });
    let info_height = if is_downloading { 12 } else { 11 };

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(info_height), // Top section: Cover Art + Metadata
            Constraint::Min(6),              // Bottom section: Lyrics / Prompt scrollable
        ])
        .margin(1)
        .split(area);

    f.render_widget(block, area);

    // Top section: [Padding(1), Cover Art square (20 cols x 11 rows -> exact 18x9 inner = 1:1), Spacer(2), Metadata]
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),  // 1 col padding from outer left border
            Constraint::Length(20), // Cover Art preview box (20 cols x 11 rows)
            Constraint::Length(2),  // Spacer between cover art and metadata
            Constraint::Min(20),    // Track metadata
        ])
        .split(inner_chunks[0]);

    let cover_area = top_chunks[1];
    let meta_area = top_chunks[3];

    // 1. Cover Art Box
    let is_cover_hovered = is_hovered(app.mouse_pos, cover_area);
    let cover_border_color = if is_cover_hovered {
        Color::Cyan
    } else {
        Color::Rgb(70, 80, 100)
    };

    let cover_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(cover_border_color))
        .title(Span::styled(
            format!("{}Cover Art", app.icons.music),
            Style::default().fg(if is_cover_hovered {
                Color::Cyan
            } else {
                Color::DarkGray
            }),
        ));

    let inner_cover = cover_block.inner(cover_area);
    f.render_widget(cover_block, cover_area);

    // Ensure cover art is queued for loading if not already cached or requested
    if !app.cover_cache.contains_key(&item.clip.id) && !app.pending_covers.contains(&item.clip.id) {
        app.ensure_current_cover();
    }

    if let Some(proto) = app.cover_cache.get_mut(&item.clip.id) {
        let image_widget = ratatui_image::StatefulImage::new(None);
        f.render_stateful_widget(image_widget, inner_cover, proto);
    } else if app.pending_covers.contains(&item.clip.id) {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinner_frames[app.anim_frame % spinner_frames.len()];
        let loading_p = Paragraph::new(format!("\n\n\n   {} Loading\n     Cover...", spinner))
            .alignment(Alignment::Left)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(loading_p, inner_cover);
    } else {
        let placeholder = Paragraph::new(format!("\n\n\n     {}", app.icons.music))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(placeholder, inner_cover);
    }

    // 2. Metadata details & contextual buttons
    let clip = &item.clip;
    let status_span = match &item.download_status {
        DownloadStatus::Idle => Span::styled("Ready to download", Style::default().fg(Color::Gray)),
        DownloadStatus::Queued => Span::styled(
            "In download queue...",
            Style::default().fg(Color::Rgb(251, 191, 36)),
        ),
        DownloadStatus::Downloading {
            progress,
            downloaded_bytes,
            total_bytes,
        } => {
            let pct = (progress * 100.0) as u32;
            let mb_dl = *downloaded_bytes as f64 / 1_048_576.0;
            let mb_tot = *total_bytes as f64 / 1_048_576.0;
            Span::styled(
                format!("Downloading {}% ({:.1} / {:.1} MB)", pct, mb_dl, mb_tot),
                Style::default()
                    .fg(Color::Rgb(56, 189, 248))
                    .add_modifier(Modifier::BOLD),
            )
        }
        DownloadStatus::Completed { path } => Span::styled(
            format!("{}Saved: {}", app.icons.check, path.display()),
            Style::default()
                .fg(Color::Rgb(74, 222, 128))
                .add_modifier(Modifier::BOLD),
        ),
        DownloadStatus::Failed { error } => Span::styled(
            format!("{}Error: {}", app.icons.cross, error),
            Style::default()
                .fg(Color::Rgb(248, 113, 113))
                .add_modifier(Modifier::BOLD),
        ),
    };

    let btn_rects = get_inspector_button_rects(area);
    let is_play_hovered = is_hovered(app.mouse_pos, btn_rects.play_rect);
    let is_stop_hovered = is_hovered(app.mouse_pos, btn_rects.stop_rect);
    let is_download_hovered = is_hovered(app.mouse_pos, btn_rects.download_rect);
    let is_select_hovered = is_hovered(app.mouse_pos, btn_rects.select_rect);

    let is_currently_playing = app.player.current_id() == Some(&clip.id) && app.player.is_playing();
    let is_loading_audio = app.loading_audio_id.as_deref() == Some(&clip.id);

    let (play_button_span, stop_button_span) = if app.player.backend() == PlayerBackend::None {
        (
            Span::styled(
                " [ ⚠ Player Missing ] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(if is_play_hovered {
                        Color::Rgb(254, 240, 138)
                    } else {
                        Color::Rgb(251, 191, 36)
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " [ ⏹ Stop (s) ] ",
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(Color::Rgb(35, 35, 40)),
            ),
        )
    } else {
        let stop_span = Span::styled(
            " [ ⏹ Stop (s) ] ",
            Style::default()
                .fg(Color::White)
                .bg(if is_stop_hovered {
                    Color::Rgb(127, 29, 29)
                } else {
                    Color::Rgb(60, 25, 25)
                })
                .add_modifier(if is_stop_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        );

        let play_span = if is_loading_audio {
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner = spinner_frames[app.anim_frame % spinner_frames.len()];
            Span::styled(
                format!(" [ {} Preparing... ] ", spinner),
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Rgb(20, 45, 65))
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_currently_playing {
            Span::styled(
                " [ ⏸ Pause (p) ] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(if is_play_hovered {
                        Color::Rgb(254, 240, 138)
                    } else {
                        Color::Yellow
                    })
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " [ ▶ Play (p) ] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(if is_play_hovered {
                        Color::Rgb(134, 239, 172)
                    } else {
                        Color::Green
                    })
                    .add_modifier(Modifier::BOLD),
            )
        };

        (play_span, stop_span)
    };

    let download_button_span = Span::styled(
        " [ ⤓ Download (d) ] ",
        Style::default()
            .fg(if is_download_hovered {
                Color::White
            } else {
                Color::Rgb(74, 222, 128)
            })
            .bg(if is_download_hovered {
                Color::Rgb(22, 101, 52)
            } else {
                Color::Rgb(20, 50, 30)
            })
            .add_modifier(if is_download_hovered {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
    );

    let select_button_span = if item.selected {
        Span::styled(
            " [ ✓ Selected (Space) ] ",
            Style::default()
                .fg(if is_select_hovered {
                    Color::Black
                } else {
                    Color::Rgb(250, 204, 21)
                })
                .bg(if is_select_hovered {
                    Color::Rgb(234, 179, 8)
                } else {
                    Color::Rgb(60, 48, 18)
                })
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ ○ Select (Space) ] ",
            Style::default()
                .fg(if is_select_hovered {
                    Color::White
                } else {
                    Color::Rgb(147, 197, 253)
                })
                .bg(if is_select_hovered {
                    Color::Rgb(37, 99, 235)
                } else {
                    Color::Rgb(25, 40, 60)
                })
                .add_modifier(if is_select_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        )
    };

    let meta_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} Title:    ", app.icons.music),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                clip.display_title(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{} Artist:   ", app.icons.artist),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                clip.display_artist(),
                Style::default()
                    .fg(Color::Rgb(56, 189, 248))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   {} Duration: ", app.icons.clock),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                clip.duration_formatted(),
                Style::default().fg(Color::Rgb(250, 204, 21)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{} Model:    ", app.icons.lightning),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                clip.display_model(),
                Style::default()
                    .fg(Color::Rgb(74, 222, 128))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Quality: ", Style::default().fg(Color::DarkGray)),
            if crate::player::which("ffmpeg") {
                Span::styled("320 kbps MP3", Style::default().fg(Color::LightCyan))
            } else {
                Span::styled(
                    "Original M4A (ffmpeg missing)",
                    Style::default().fg(Color::Rgb(251, 191, 36)),
                )
            },
        ]),
        Line::from(vec![
            Span::styled(
                format!("{} Tags:     ", app.icons.tag),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                clip.display_tags(),
                Style::default().fg(Color::Rgb(200, 200, 210)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Status:   ", Style::default().fg(Color::DarkGray)),
            status_span,
        ]),
        Line::from(vec![
            Span::styled("  Audio:    ", Style::default().fg(Color::DarkGray)),
            play_button_span,
            Span::raw(" "),
            stop_button_span,
            Span::raw(" "),
            Span::styled(
                format!("({})", app.player.backend().name()),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Action:   ", Style::default().fg(Color::DarkGray)),
            download_button_span,
            Span::raw(" "),
            select_button_span,
        ]),
    ];

    if let DownloadStatus::Downloading {
        progress,
        downloaded_bytes,
        total_bytes,
    } = &item.download_status
    {
        let pct = (*progress * 100.0).clamp(0.0, 100.0) as u16;
        let mb_dl = *downloaded_bytes as f64 / 1_048_576.0;
        let mb_tot = *total_bytes as f64 / 1_048_576.0;
        let label = format!("{}% ({:.1} MB / {:.1} MB)", pct, mb_dl, mb_tot);

        let meta_p = Paragraph::new(meta_lines);
        let meta_rect = Rect {
            x: meta_area.x,
            y: meta_area.y,
            width: meta_area.width,
            height: 7,
        };
        f.render_widget(meta_p, meta_rect);

        let gauge_rect = Rect {
            x: meta_area.x,
            y: meta_area.y + 7,
            width: meta_area.width,
            height: 1,
        };
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(Color::Rgb(56, 189, 248))
                    .bg(Color::Rgb(30, 40, 55)),
            )
            .percent(pct)
            .label(Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
        f.render_widget(gauge, gauge_rect);
    } else {
        let meta_p = Paragraph::new(meta_lines);
        f.render_widget(meta_p, meta_area);
    }

    // 3. Lyrics / Prompt Box with styled section tags
    let is_lyrics_hovered = is_hovered(app.mouse_pos, inner_chunks[1]);
    let lyrics_border_color = if is_lyrics_hovered {
        Color::Cyan
    } else {
        Color::Rgb(70, 75, 90)
    };

    let lyrics_title = format!(
        " {}Lyrics & Generation Prompt [Scroll ↕ Line {} | Mouse Wheel] ",
        app.icons.lyrics, app.details_scroll
    );
    let lyrics_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(lyrics_border_color))
        .title(Span::styled(
            lyrics_title,
            Style::default().fg(Color::Rgb(180, 185, 200)),
        ));

    let prompt_raw = clip.display_prompt();
    let mut lyrics_lines = Vec::new();
    for line in prompt_raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            lyrics_lines.push(Line::from(Span::styled(
                line,
                Style::default()
                    .fg(Color::Rgb(250, 204, 21))
                    .add_modifier(Modifier::BOLD),
            )));
        } else {
            lyrics_lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::Rgb(220, 225, 235)),
            )));
        }
    }

    let lyrics_p = Paragraph::new(lyrics_lines)
        .block(lyrics_block)
        .wrap(Wrap { trim: false })
        .scroll((app.details_scroll, 0));

    f.render_widget(lyrics_p, inner_chunks[1]);
}

fn render_footer(f: &mut Frame, app: &mut App, area: Rect, dimmed: bool) {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if dimmed {
            Color::Rgb(40, 50, 65)
        } else {
            Color::Rgb(70, 80, 100)
        }));

    // If there is an active status message, display it cleanly in the footer border
    if let Some(msg) = &app.status_msg {
        block = block.title(Line::from(vec![Span::styled(
            format!(" 💬 {} ", msg.text),
            Style::default()
                .fg(msg.color)
                .bg(Color::Rgb(20, 25, 35))
                .add_modifier(Modifier::BOLD),
        )]));
    }

    f.render_widget(block, area);

    let hitboxes = get_footer_hitboxes(area);

    // 1. LEFT: [q] Quit
    let is_quit_hovered = is_hovered(app.mouse_pos, hitboxes.quit_rect);
    let quit_line = Line::from(vec![
        Span::styled(
            "[q]",
            Style::default()
                .fg(if is_quit_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(248, 113, 113)
                })
                .bg(if is_quit_hovered && !dimmed {
                    Color::Rgb(127, 29, 29)
                } else {
                    Color::Reset
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Quit",
            Style::default()
                .fg(if is_quit_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(209, 213, 219)
                })
                .bg(if is_quit_hovered && !dimmed {
                    Color::Rgb(127, 29, 29)
                } else {
                    Color::Reset
                })
                .add_modifier(if is_quit_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
    ]);
    f.render_widget(Paragraph::new(quit_line), hitboxes.quit_rect);

    // 2. CENTER: [a] Add Track • [/] Search • [r] Sync
    let is_add_hovered = is_hovered(app.mouse_pos, hitboxes.add_rect);
    let is_search_hovered = is_hovered(app.mouse_pos, hitboxes.search_rect);
    let is_refresh_hovered = is_hovered(app.mouse_pos, hitboxes.refresh_rect);

    let sep = Span::styled(" • ", Style::default().fg(Color::Rgb(75, 85, 100)));

    let center_line = Line::from(vec![
        Span::styled(
            "[a]",
            Style::default()
                .fg(if is_add_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(56, 189, 248)
                })
                .bg(if is_add_hovered && !dimmed {
                    Color::Rgb(30, 58, 95)
                } else {
                    Color::Reset
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Add Track",
            Style::default()
                .fg(if is_add_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(209, 213, 219)
                })
                .bg(if is_add_hovered && !dimmed {
                    Color::Rgb(30, 58, 95)
                } else {
                    Color::Reset
                })
                .add_modifier(if is_add_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        sep.clone(),
        Span::styled(
            "[/]",
            Style::default()
                .fg(if is_search_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(250, 204, 21)
                })
                .bg(if is_search_hovered && !dimmed {
                    Color::Rgb(70, 55, 20)
                } else {
                    Color::Reset
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Search",
            Style::default()
                .fg(if is_search_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(209, 213, 219)
                })
                .bg(if is_search_hovered && !dimmed {
                    Color::Rgb(70, 55, 20)
                } else {
                    Color::Reset
                })
                .add_modifier(if is_search_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        sep,
        Span::styled(
            "[r]",
            Style::default()
                .fg(if is_refresh_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(192, 132, 252)
                })
                .bg(if is_refresh_hovered && !dimmed {
                    Color::Rgb(65, 30, 75)
                } else {
                    Color::Reset
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Sync",
            Style::default()
                .fg(if is_refresh_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(209, 213, 219)
                })
                .bg(if is_refresh_hovered && !dimmed {
                    Color::Rgb(65, 30, 75)
                } else {
                    Color::Reset
                })
                .add_modifier(if is_refresh_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
    ]);
    let center_area = Rect {
        x: hitboxes.add_rect.x,
        y: hitboxes.add_rect.y,
        width: 37,
        height: 1,
    };
    f.render_widget(Paragraph::new(center_line), center_area);

    // 3. RIGHT: [c] Settings • [?] Help
    let is_settings_hovered = is_hovered(app.mouse_pos, hitboxes.settings_rect);
    let is_help_hovered = is_hovered(app.mouse_pos, hitboxes.help_rect);

    let right_line = Line::from(vec![
        Span::styled(
            "[c]",
            Style::default()
                .fg(if is_settings_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(147, 197, 253)
                })
                .bg(if is_settings_hovered && !dimmed {
                    Color::Rgb(30, 45, 65)
                } else {
                    Color::Reset
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Settings",
            Style::default()
                .fg(if is_settings_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(209, 213, 219)
                })
                .bg(if is_settings_hovered && !dimmed {
                    Color::Rgb(30, 45, 65)
                } else {
                    Color::Reset
                })
                .add_modifier(if is_settings_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::styled(" • ", Style::default().fg(Color::Rgb(75, 85, 100))),
        Span::styled(
            "[?]",
            Style::default()
                .fg(if is_help_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(220, 225, 235)
                })
                .bg(if is_help_hovered && !dimmed {
                    Color::Rgb(45, 45, 55)
                } else {
                    Color::Reset
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Help",
            Style::default()
                .fg(if is_help_hovered && !dimmed {
                    Color::White
                } else {
                    Color::Rgb(209, 213, 219)
                })
                .bg(if is_help_hovered && !dimmed {
                    Color::Rgb(45, 45, 55)
                } else {
                    Color::Reset
                })
                .add_modifier(if is_help_hovered {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
    ]);
    let right_area = Rect {
        x: hitboxes.settings_rect.x,
        y: hitboxes.settings_rect.y,
        width: 23,
        height: 1,
    };
    f.render_widget(Paragraph::new(right_line), right_area);
}

fn render_add_url_modal(f: &mut Frame, app: &App) {
    let area = add_url_modal_rect(f.area());

    // Dimmed background backdrop
    f.render_widget(Clear, area);

    let modal_bg = Color::Rgb(18, 24, 34);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(modal_bg))
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            format!(" {}Add Suno Track (URL or UUID) ", app.icons.add),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Prompt text
            Constraint::Length(3), // Input box
            Constraint::Length(2), // Clickable Action Buttons
        ])
        .margin(1)
        .split(area);

    f.render_widget(block, area);

    let prompt = Paragraph::new(
        "Paste a Suno song share link (e.g. https://suno.com/song/...) or raw UUID:",
    )
    .style(Style::default().fg(Color::White).bg(modal_bg));
    f.render_widget(prompt, inner_chunks[0]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(14, 18, 26)))
        .border_style(Style::default().fg(Color::Cyan));

    let input_text = Line::from(vec![
        Span::styled(
            &app.url_input,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("▍", Style::default().fg(Color::Yellow)),
    ]);
    let input_p = Paragraph::new(input_text).block(input_block);
    f.render_widget(input_p, inner_chunks[1]);

    let is_add_hovered = is_hovered(
        app.mouse_pos,
        Rect {
            x: area.x + 4,
            y: inner_chunks[2].y,
            width: 28,
            height: 1,
        },
    );
    let is_cancel_hovered = is_hovered(
        app.mouse_pos,
        Rect {
            x: area.x + 35,
            y: inner_chunks[2].y,
            width: 16,
            height: 1,
        },
    );

    let buttons_line = Line::from(vec![
        Span::styled(
            format!(" [ ↵ {}Download / Add Track ] ", app.icons.download),
            Style::default()
                .fg(Color::Black)
                .bg(if is_add_hovered {
                    Color::Rgb(100, 240, 150)
                } else {
                    Color::Green
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled(
            format!(" [ ⎋ {}Cancel ] ", app.icons.quit),
            Style::default().fg(Color::White).bg(if is_cancel_hovered {
                Color::Rgb(180, 50, 50)
            } else {
                Color::DarkGray
            }),
        ),
    ]);
    let hints = Paragraph::new(buttons_line)
        .alignment(Alignment::Center)
        .style(Style::default().bg(modal_bg));
    f.render_widget(hints, inner_chunks[2]);
}

fn render_settings_modal(f: &mut Frame, app: &App) {
    let area = settings_modal_rect(f.area());

    // Dimmed background
    f.render_widget(Clear, area);

    let modal_bg = Color::Rgb(16, 20, 28);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(modal_bg))
        .border_style(Style::default().fg(Color::Rgb(74, 222, 128)))
        .title(Span::styled(
            format!(" {}Settings & Preferences ", app.icons.gear),
            Style::default()
                .fg(Color::Rgb(74, 222, 128))
                .add_modifier(Modifier::BOLD),
        ));

    f.render_widget(block, area);

    let inner_chunks = settings_inner_chunks(area);

    // 0: Nav tips
    let nav_tips = Line::from(vec![
        Span::styled(" Navigate: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[Tab]/[↑↓]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  Toggle: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[Space]/[Click]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  Auto-Detect: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[t]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  Save: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[Ctrl+S]/[Enter]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let tips_p = Paragraph::new(nav_tips).style(Style::default().bg(modal_bg));
    f.render_widget(tips_p, inner_chunks[0]);

    // 1: Token input box
    let is_token_focused = app.settings_field == SettingsField::Token;
    let is_token_hovered = is_hovered(app.mouse_pos, inner_chunks[1]);
    let token_border = if is_token_focused {
        Color::Cyan
    } else if is_token_hovered {
        Color::Yellow
    } else {
        Color::Rgb(70, 80, 100)
    };

    let token_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(12, 16, 24)))
        .border_style(Style::default().fg(token_border))
        .title(Span::styled(
            if is_token_focused {
                " ▶ 1. Suno Session Token (JWT / Cookie) [Active] "
            } else {
                " 1. Suno Session Token (JWT / Cookie) "
            },
            Style::default()
                .fg(token_border)
                .add_modifier(if is_token_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ));

    let display_token = if app.settings_token_input.is_empty() {
        Span::styled(
            "<Empty - Click or press 't' on Auto-Detect below>",
            Style::default().fg(Color::DarkGray),
        )
    } else {
        let len = app.settings_token_input.len();
        let preview = if len > 50 {
            format!(
                "{}...{} ({} chars)",
                &app.settings_token_input[..25],
                &app.settings_token_input[len - 15..],
                len
            )
        } else {
            app.settings_token_input.clone()
        };
        Span::styled(
            preview,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
    };

    let mut token_spans = vec![display_token];
    if is_token_focused {
        token_spans.push(Span::styled("▍", Style::default().fg(Color::Yellow)));
    }
    let token_p = Paragraph::new(Line::from(token_spans)).block(token_block);
    f.render_widget(token_p, inner_chunks[1]);

    // 2: Auto-detect button
    let is_ad_focused = app.settings_field == SettingsField::AutoDetectBtn;
    let is_ad_hovered = is_hovered(app.mouse_pos, inner_chunks[2]);
    let ad_active = is_ad_focused || is_ad_hovered;
    let ad_span = Span::styled(
        " [ ⚡ Auto-Detect Token from Chrome / KWallet / Firefox (Press 't') ] ",
        if ad_active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 35, 50))
        },
    );
    let ad_p = Paragraph::new(Line::from(vec![
        if is_ad_focused {
            Span::styled("▶ ", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("  ")
        },
        ad_span,
    ]))
    .style(Style::default().bg(modal_bg));
    f.render_widget(ad_p, inner_chunks[2]);

    // 3: Download Directory input block
    let is_dir_focused = app.settings_field == SettingsField::DownloadDir;
    let is_dir_hovered = is_hovered(app.mouse_pos, inner_chunks[3]);
    let dir_border = if is_dir_focused {
        Color::Cyan
    } else if is_dir_hovered {
        Color::Yellow
    } else {
        Color::Rgb(70, 80, 100)
    };

    let dir_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(12, 16, 24)))
        .border_style(Style::default().fg(dir_border))
        .title(Span::styled(
            if is_dir_focused {
                " ▶ 2. Download Destination Folder [Active] "
            } else {
                " 2. Download Destination Folder "
            },
            Style::default()
                .fg(dir_border)
                .add_modifier(if is_dir_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ));

    let mut dir_spans = vec![Span::styled(
        &app.settings_download_dir_input,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];
    if is_dir_focused {
        dir_spans.push(Span::styled("▍", Style::default().fg(Color::Yellow)));
    }
    let dir_p = Paragraph::new(Line::from(dir_spans)).block(dir_block);
    f.render_widget(dir_p, inner_chunks[3]);

    // 4: Filename Template input block
    let is_tpl_focused = app.settings_field == SettingsField::FileTemplate;
    let is_tpl_hovered = is_hovered(app.mouse_pos, inner_chunks[4]);
    let tpl_border = if is_tpl_focused {
        Color::Cyan
    } else if is_tpl_hovered {
        Color::Yellow
    } else {
        Color::Rgb(70, 80, 100)
    };

    let tpl_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Rgb(12, 16, 24)))
        .border_style(Style::default().fg(tpl_border))
        .title(Span::styled(
            if is_tpl_focused {
                " ▶ 3. File Naming Pattern ({title}, {id}, {artist}, {style}) [Active] "
            } else {
                " 3. File Naming Pattern "
            },
            Style::default()
                .fg(tpl_border)
                .add_modifier(if is_tpl_focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ));

    let mut tpl_spans = vec![Span::styled(
        &app.settings_file_template_input,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];
    if is_tpl_focused {
        tpl_spans.push(Span::styled("▍", Style::default().fg(Color::Yellow)));
    }
    let tpl_p = Paragraph::new(Line::from(tpl_spans)).block(tpl_block);
    f.render_widget(tpl_p, inner_chunks[4]);

    // 5: Concurrency selector
    let is_conc_focused = app.settings_field == SettingsField::Concurrency;
    let conc_line = Line::from(vec![
        if is_conc_focused {
            Span::styled("▶ ", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("  ")
        },
        Span::styled(
            "4. Simultaneous Downloads: ",
            Style::default()
                .fg(if is_conc_focused {
                    Color::White
                } else {
                    Color::DarkGray
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ◄ ",
            Style::default()
                .fg(if app.settings_concurrency > 1 {
                    Color::Cyan
                } else {
                    Color::DarkGray
                })
                .bg(Color::Rgb(30, 41, 59))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} parallel  ", app.settings_concurrency),
            Style::default()
                .fg(if is_conc_focused {
                    Color::White
                } else {
                    Color::Rgb(226, 232, 240)
                })
                .bg(if is_conc_focused {
                    Color::Rgb(30, 58, 88)
                } else {
                    Color::Rgb(15, 23, 42)
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ► ",
            Style::default()
                .fg(if app.settings_concurrency < 10 {
                    Color::Cyan
                } else {
                    Color::DarkGray
                })
                .bg(Color::Rgb(30, 41, 59))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (Range: 1..10, use ← / →, Space or Click to adjust)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let conc_p = Paragraph::new(conc_line).style(Style::default().bg(modal_bg));
    f.render_widget(conc_p, inner_chunks[5]);

    // 6: Terminal Graphics Protocol selector
    let is_gfx_focused = app.settings_field == SettingsField::GraphicsProtocol;
    let proto_name = app.image_protocol_label();
    let gfx_line = Line::from(vec![
        if is_gfx_focused {
            Span::styled("▶ ", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("  ")
        },
        Span::styled(
            "5. Terminal Graphics Protocol: ",
            Style::default()
                .fg(if is_gfx_focused {
                    Color::White
                } else {
                    Color::DarkGray
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ◄ ",
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Rgb(30, 41, 59))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}  ", proto_name),
            Style::default()
                .fg(if is_gfx_focused {
                    Color::White
                } else {
                    Color::Rgb(226, 232, 240)
                })
                .bg(if is_gfx_focused {
                    Color::Rgb(30, 58, 88)
                } else {
                    Color::Rgb(15, 23, 42)
                })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ► ",
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Rgb(30, 41, 59))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (Auto-detected, use ← / →, Space or Click to switch)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let gfx_p = Paragraph::new(gfx_line).style(Style::default().bg(modal_bg));
    f.render_widget(gfx_p, inner_chunks[6]);

    // 7: Modern Toggle Switches
    let is_id3_focused = app.settings_field == SettingsField::EmbedId3;
    let is_sc_focused = app.settings_field == SettingsField::SaveCover;
    let is_sl_focused = app.settings_field == SettingsField::SaveLyrics;

    let make_toggle_line = |is_on: bool, is_focused: bool, title: &str, subtitle: &str| -> Line {
        let toggle_span = if is_on {
            Span::styled(
                " [● ON] ",
                Style::default()
                    .fg(Color::Rgb(15, 23, 42))
                    .bg(if is_focused {
                        Color::Rgb(74, 222, 128)
                    } else {
                        Color::Rgb(34, 197, 94)
                    })
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " [○ OFF] ",
                Style::default()
                    .fg(if is_focused {
                        Color::White
                    } else {
                        Color::Rgb(156, 163, 175)
                    })
                    .bg(if is_focused {
                        Color::Rgb(55, 65, 81)
                    } else {
                        Color::Rgb(30, 41, 59)
                    })
                    .add_modifier(if is_focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )
        };

        let pointer = if is_focused {
            Span::styled(
                " ▶ ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("   ")
        };

        let title_style = if is_focused {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(226, 232, 240))
        };

        let mut spans = vec![
            pointer,
            toggle_span,
            Span::styled(format!(" {} ", title), title_style),
        ];

        if is_focused {
            spans.push(Span::styled(
                format!("({}) [Space/Click to Toggle]", subtitle),
                Style::default().fg(Color::Rgb(250, 204, 21)),
            ));
        } else {
            spans.push(Span::styled(
                format!("({})", subtitle),
                Style::default().fg(Color::DarkGray),
            ));
        }

        Line::from(spans)
    };

    let cb_lines = vec![
        make_toggle_line(
            app.settings_embed_id3,
            is_id3_focused,
            "Embed ID3v2.3 Tags & Cover Art into MP3",
            "Title, Artist, Lyrics, Front Cover",
        ),
        make_toggle_line(
            app.settings_save_cover,
            is_sc_focused,
            "Save Standalone Cover Image File",
            ".jpg / .png",
        ),
        make_toggle_line(
            app.settings_save_lyrics,
            is_sl_focused,
            "Save Standalone Lyrics & Prompt File",
            ".txt",
        ),
    ];
    let cb_p = Paragraph::new(cb_lines).style(Style::default().bg(modal_bg));
    f.render_widget(cb_p, inner_chunks[7]);

    // 8: System Diagnostics & Terminal Graphics status line
    let mpv_ready = app.player.backend() != PlayerBackend::None;
    let ffmpeg_ready = crate::player::which("ffmpeg");
    let token_ready = app
        .config
        .session_token
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let mpv_span = if mpv_ready {
        Span::styled(
            format!(" [{}mpv] ", app.icons.check),
            Style::default().fg(Color::Green),
        )
    } else {
        Span::styled(
            format!(" [{}mpv missing] ", app.icons.cross),
            Style::default().fg(Color::Rgb(251, 191, 36)),
        )
    };
    let ffmpeg_span = if ffmpeg_ready {
        Span::styled(
            format!(" [{}ffmpeg] ", app.icons.check),
            Style::default().fg(Color::Green),
        )
    } else {
        Span::styled(
            format!(" [{}ffmpeg missing] ", app.icons.cross),
            Style::default().fg(Color::Rgb(251, 191, 36)),
        )
    };
    let proto_span = Span::styled(
        format!(
            " [{}Graphics: {}] ",
            app.icons.check,
            app.image_protocol_label()
        ),
        Style::default().fg(Color::Green),
    );
    let token_span = if token_ready {
        Span::styled(
            format!(" [{}Token active] ", app.icons.shield_auth),
            Style::default().fg(Color::Green),
        )
    } else {
        Span::styled(
            format!(" [{}Anonymous / No token] ", app.icons.shield_anon),
            Style::default().fg(Color::Rgb(251, 191, 36)),
        )
    };

    let sys_line = Line::from(vec![
        Span::styled("  Status: ", Style::default().fg(Color::DarkGray)),
        mpv_span,
        ffmpeg_span,
        proto_span,
        token_span,
    ]);
    let sys_p = Paragraph::new(sys_line).style(Style::default().bg(modal_bg));
    f.render_widget(sys_p, inner_chunks[8]);

    // 9: Action buttons
    let is_save_focused = app.settings_field == SettingsField::SaveBtn;
    let is_cancel_focused = app.settings_field == SettingsField::CancelBtn;
    let mid_x = area.x + area.width / 2;
    let is_save_hovered = is_hovered(
        app.mouse_pos,
        Rect {
            x: area.x + 4,
            y: inner_chunks[9].y,
            width: (area.width / 2).saturating_sub(6),
            height: 1,
        },
    );
    let is_cancel_hovered = is_hovered(
        app.mouse_pos,
        Rect {
            x: mid_x + 2,
            y: inner_chunks[9].y,
            width: (area.width / 2).saturating_sub(6),
            height: 1,
        },
    );

    let save_active = is_save_focused || is_save_hovered;
    let cancel_active = is_cancel_focused || is_cancel_hovered;

    let save_bg = if save_active {
        Color::Rgb(100, 240, 150)
    } else {
        Color::Green
    };
    let cancel_bg = if cancel_active {
        Color::Rgb(180, 50, 50)
    } else {
        Color::DarkGray
    };

    let buttons_line = Line::from(vec![
        Span::styled(
            format!(" [ ↵ {}Save Configuration ] ", app.icons.check),
            Style::default()
                .fg(Color::Black)
                .bg(save_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled(
            format!(" [ ⎋ {}Cancel ] ", app.icons.quit),
            Style::default().fg(Color::White).bg(cancel_bg),
        ),
    ]);
    let actions = Paragraph::new(buttons_line)
        .alignment(Alignment::Center)
        .style(Style::default().bg(modal_bg));
    f.render_widget(actions, inner_chunks[9]);
}

fn render_help_modal(f: &mut Frame, app: &App) {
    let area = help_modal_rect(f.area());

    // Dimmed background
    f.render_widget(Clear, area);

    let modal_bg = Color::Rgb(18, 24, 34);

    let font_status = if app.icons.is_nerd {
        "Nerd Fonts (Auto-detected active)"
    } else {
        "Standard Unicode"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(modal_bg))
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            format!(
                " {}Controls & Help [Icon Mode: {}] ",
                app.icons.help, font_status
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let is_close_hovered = is_hovered(
        app.mouse_pos,
        Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height.saturating_sub(2),
            width: area.width / 2,
            height: 1,
        },
    );

    let help_lines = vec![
        Line::from(Span::styled(
            "🖱 Mouse Controls:",
            Style::default()
                .fg(Color::Rgb(56, 189, 248))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  • Click Track Row     Select & inspect track (loads cover art)"),
        Line::from("  • Click [ ] Checkbox  Toggle track selection for batch download"),
        Line::from("  • Click Art Box       Toggle cover graphics (Sixel ↔ Halfblocks)"),
        Line::from("  • Mouse Wheel Up/Down Scroll track list or scroll lyrics inspector"),
        Line::from("  • Click Footer Pills  Directly trigger actions (Add, Download, Play, etc.)"),
        Line::from("  • Hover over buttons  Buttons & rows light up interactively"),
        Line::from(""),
        Line::from(Span::styled(
            "⌨️ Keyboard Shortcuts (RU/Cyrillic keyboard layouts fully supported):",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑ / ↓, k / j         Move cursor up / down"),
        Line::from("  p / s                Play audio stream / Stop playback"),
        Line::from("  i                    Toggle cover graphics (also configurable in Settings [c])"),
        Line::from("  Space                Toggle selection on current track"),
        Line::from("  Ctrl+A / Ctrl+D      Select all / Deselect all tracks"),
        Line::from("  d                    Download selected (or current) track(s)"),
        Line::from("  a, u                 Add track by public URL or UUID"),
        Line::from("  r                    Refresh & sync library from Suno"),
        Line::from("  /                    Filter / search tracks (Esc to clear)"),
        Line::from("  c                    Configure session token & settings"),
        Line::from("  Ctrl+S               (In Settings) Save & apply settings immediately"),
        Line::from("  t                    (In Settings) Auto-detect token from browser"),
        Line::from("  J / K                Scroll lyrics & prompt down / up"),
        Line::from("  q, Esc               Quit / Close dialog"),
        Line::from(""),
        Line::from(Span::styled(
            "📦 System Dependencies & Installation:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  • mpv:    In-TUI audio playback (sudo apt install mpv / pacman -S mpv / brew install mpv)"),
        Line::from("  • ffmpeg: 320 kbps MP3 transcoding (sudo apt install ffmpeg / pacman -S ffmpeg)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!(" [ ⎋ {}Close Help (Click or Esc) ] ", app.icons.quit),
            Style::default()
                .fg(Color::Black)
                .bg(if is_close_hovered {
                    Color::White
                } else {
                    Color::Cyan
                })
                .add_modifier(Modifier::BOLD),
        )]),
    ];

    let p = Paragraph::new(help_lines)
        .block(block)
        .style(Style::default().fg(Color::White).bg(modal_bg))
        .alignment(Alignment::Left);

    f.render_widget(p, area);
}
