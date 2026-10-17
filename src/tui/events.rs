use crate::tui::app::{ActiveModal, App, SettingsField};
use crate::tui::ui::{
    add_url_modal_rect, get_footer_hitboxes, get_inspector_button_rects, is_hovered,
    settings_inner_chunks, settings_modal_rect,
};
use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;
use std::time::Duration;

pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    mut app: App,
) -> Result<()> {
    let tick_rate = Duration::from_millis(50);

    // Initial check: if authorized, load personal library
    if app
        .config
        .session_token
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        app.refresh_library();
    }

    loop {
        app.clear_expired_status();
        app.update_channels();

        terminal.draw(|f| {
            super::ui::render(f, &mut app);
        })?;

        if app.should_quit {
            break;
        }

        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key_event(&mut app, key);
                }
                Event::Mouse(mouse) => {
                    app.mouse_pos = Some((mouse.column, mouse.row));
                    let size = terminal.size()?;
                    handle_mouse_event(&mut app, mouse, (size.width, size.height));
                }
                _ => {}
            }
        }
    }

    // Stop any active audio player before exiting terminal
    app.player.stop();

    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    // 1. Modals handling
    match app.active_modal {
        ActiveModal::AddUrl => {
            handle_add_url_keys(app, key);
            return;
        }
        ActiveModal::Settings => {
            handle_settings_keys(app, key);
            return;
        }
        ActiveModal::Help => {
            let norm_code = match key.code {
                KeyCode::Char(c) => KeyCode::Char(normalize_cyrillic(c)),
                other => other,
            };
            match norm_code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') | KeyCode::Enter => {
                    app.active_modal = ActiveModal::None;
                }
                _ => {}
            }
            return;
        }
        ActiveModal::None => {}
    }

    // 2. Search mode handling (keep raw characters for Cyrillic and international queries)
    if app.search_mode {
        match key.code {
            KeyCode::Enter => {
                app.search_mode = false;
            }
            KeyCode::Esc => {
                app.search_query.clear();
                app.search_mode = false;
                app.selected_index = 0;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.selected_index = 0;
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.selected_index = 0;
            }
            _ => {}
        }
        return;
    }

    // 3. Normal navigation & action keys (normalized for Cyrillic keyboard layouts)
    let norm_code = match key.code {
        KeyCode::Char(c) => KeyCode::Char(normalize_cyrillic(c)),
        other => other,
    };

    match norm_code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.search_query.clear();
                app.selected_index = 0;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_cursor_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_cursor_down();
        }
        KeyCode::PageUp => {
            app.page_up(10);
        }
        KeyCode::PageDown => {
            app.page_down(10);
        }
        KeyCode::Home => {
            app.move_cursor_first();
        }
        KeyCode::End => {
            app.move_cursor_last();
        }
        KeyCode::Char('J') => {
            app.scroll_details_down();
        }
        KeyCode::Char('K') => {
            app.scroll_details_up();
        }
        KeyCode::Char(' ') => {
            app.toggle_select_current();
        }
        KeyCode::Char('p') => {
            app.play_current_track();
        }
        KeyCode::Char('s') => {
            app.stop_playback();
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.select_all_visible();
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.deselect_all();
        }
        KeyCode::Char('a') | KeyCode::Char('u') => {
            app.url_input.clear();
            app.active_modal = ActiveModal::AddUrl;
        }
        KeyCode::Char('d') => {
            app.start_downloads();
        }
        KeyCode::Char('/') => {
            app.search_mode = true;
        }
        KeyCode::Char('r') => {
            app.refresh_library();
        }
        KeyCode::Char('i') => {
            app.cycle_image_protocol();
        }
        KeyCode::Char('c') | KeyCode::Char('S') => {
            app.open_settings();
        }
        KeyCode::Char('h') | KeyCode::Char('?') => {
            app.active_modal = ActiveModal::Help;
        }
        _ => {}
    }
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent, size: (u16, u16)) {
    let (cols, rows) = size;
    let window_rect = Rect::new(0, 0, cols, rows);
    let left_width = (cols * 48) / 100;

    // 1. Modal interaction
    match app.active_modal {
        ActiveModal::Help => {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                app.active_modal = ActiveModal::None;
            }
            return;
        }
        ActiveModal::AddUrl => {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let rect = add_url_modal_rect(window_rect);
                // Click outside modal closes it
                if mouse.column < rect.x
                    || mouse.column >= rect.x + rect.width
                    || mouse.row < rect.y
                    || mouse.row >= rect.y + rect.height
                {
                    app.active_modal = ActiveModal::None;
                    return;
                }
                // Check if clicked the bottom button line
                let button_row = rect.y + rect.height.saturating_sub(2);
                if mouse.row == button_row {
                    let mid_x = rect.x + rect.width / 2;
                    if mouse.column < mid_x {
                        // Clicked Add
                        let input = app.url_input.trim().to_string();
                        if !input.is_empty() {
                            app.fetch_single_clip(input);
                        }
                        app.active_modal = ActiveModal::None;
                    } else {
                        // Clicked Cancel
                        app.active_modal = ActiveModal::None;
                    }
                }
            }
            return;
        }
        ActiveModal::Settings => {
            if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                let rect = settings_modal_rect(window_rect);
                // Click outside modal closes it
                if mouse.column < rect.x
                    || mouse.column >= rect.x + rect.width
                    || mouse.row < rect.y
                    || mouse.row >= rect.y + rect.height
                {
                    app.active_modal = ActiveModal::None;
                    return;
                }

                let chunks = settings_inner_chunks(rect);
                if chunks.len() >= 10 {
                    // Chunk 1: Token input
                    if mouse.row >= chunks[1].y && mouse.row < chunks[1].y + chunks[1].height {
                        app.settings_field = SettingsField::Token;
                        return;
                    }
                    // Chunk 2: Auto-detect button
                    if mouse.row == chunks[2].y {
                        app.settings_field = SettingsField::AutoDetectBtn;
                        app.try_autodetect_browser_token();
                        return;
                    }
                    // Chunk 3: Download Directory input
                    if mouse.row >= chunks[3].y && mouse.row < chunks[3].y + chunks[3].height {
                        app.settings_field = SettingsField::DownloadDir;
                        return;
                    }
                    // Chunk 4: Filename Template input
                    if mouse.row >= chunks[4].y && mouse.row < chunks[4].y + chunks[4].height {
                        app.settings_field = SettingsField::FileTemplate;
                        return;
                    }
                    // Chunk 5: Concurrency selector
                    if mouse.row == chunks[5].y {
                        app.settings_field = SettingsField::Concurrency;
                        let mid = chunks[5].x + chunks[5].width / 2;
                        if mouse.column < mid {
                            if app.settings_concurrency > 1 {
                                app.settings_concurrency -= 1;
                            }
                        } else if app.settings_concurrency < 10 {
                            app.settings_concurrency += 1;
                        }
                        return;
                    }
                    // Chunk 6: Graphics protocol selector
                    if mouse.row == chunks[6].y {
                        app.settings_field = SettingsField::GraphicsProtocol;
                        app.cycle_image_protocol();
                        return;
                    }
                    // Chunk 7: Checkboxes
                    if mouse.row >= chunks[7].y && mouse.row < chunks[7].y + chunks[7].height {
                        let offset = mouse.row - chunks[7].y;
                        match offset {
                            0 => {
                                app.settings_field = SettingsField::EmbedId3;
                                app.settings_embed_id3 = !app.settings_embed_id3;
                            }
                            1 => {
                                app.settings_field = SettingsField::SaveCover;
                                app.settings_save_cover = !app.settings_save_cover;
                            }
                            2 => {
                                app.settings_field = SettingsField::SaveLyrics;
                                app.settings_save_lyrics = !app.settings_save_lyrics;
                            }
                            _ => {}
                        }
                        return;
                    }
                    // Chunk 9: Action buttons
                    if mouse.row >= chunks[9].y && mouse.row < chunks[9].y + chunks[9].height {
                        let mid_x = rect.x + rect.width / 2;
                        if mouse.column < mid_x {
                            app.save_settings();
                        } else {
                            app.active_modal = ActiveModal::None;
                        }
                        return;
                    }
                }
            }
            return;
        }
        ActiveModal::None => {}
    }

    // 2. Mouse Wheel Scroll
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            if mouse.column < left_width {
                app.move_cursor_down();
            } else {
                app.scroll_details_down();
            }
            return;
        }
        MouseEventKind::ScrollUp => {
            if mouse.column < left_width {
                app.move_cursor_up();
            } else {
                app.scroll_details_up();
            }
            return;
        }
        _ => {}
    }

    // 3. Left click
    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
        // A. Footer buttons (rows.saturating_sub(3)..rows)
        if mouse.row >= rows.saturating_sub(3) {
            let footer_area = Rect {
                x: 0,
                y: rows.saturating_sub(3),
                width: cols,
                height: 3,
            };
            handle_footer_click(app, footer_area, mouse.column, mouse.row);
            return;
        }

        let banner_height = if app.primary_warning().is_some() && rows > 18 {
            3
        } else {
            0
        };

        // Clicked on Warning Banner
        if banner_height > 0 && mouse.row >= 3 && mouse.row < 3 + banner_height {
            app.active_modal = ActiveModal::Help;
            return;
        }

        let body_top = 3 + banner_height;

        // B. Left panel (Track List & Search Box)
        if mouse.column < left_width && mouse.row >= body_top {
            let search_height = if app.search_mode || !app.search_query.is_empty() {
                3
            } else {
                0
            };

            // Clicked in search box
            if search_height > 0 && mouse.row < body_top + search_height {
                app.search_mode = true;
                return;
            }

            let list_top = body_top + search_height;
            let list_height = rows.saturating_sub(3 + list_top);
            let inner_height = list_height.saturating_sub(2);

            if inner_height > 0 && mouse.row > list_top && mouse.row <= list_top + inner_height {
                let row_offset = (mouse.row - (list_top + 1)) as usize;
                let vis = app.visible_indices();
                if !vis.is_empty() {
                    let selected_pos = app.selected_index.min(vis.len().saturating_sub(1));
                    let start_idx = if selected_pos >= inner_height as usize {
                        selected_pos.saturating_sub(inner_height as usize / 2)
                    } else {
                        0
                    };
                    let clicked_vis_idx = start_idx + row_offset;
                    if clicked_vis_idx < vis.len() {
                        let actual_idx = vis[clicked_vis_idx];
                        // If clicked column <= 6 (checkbox area), toggle selection
                        if mouse.column <= 6 {
                            if let Some(item) = app.items.get_mut(actual_idx) {
                                item.selected = !item.selected;
                            }
                            app.selected_index = clicked_vis_idx;
                        } else {
                            let was_already_selected = app.selected_index == clicked_vis_idx;
                            app.selected_index = clicked_vis_idx;
                            app.details_scroll = 0;
                            app.ensure_current_cover();
                            // If clicked on an already selected track, toggle playback!
                            if was_already_selected {
                                app.play_current_track();
                            }
                        }
                    }
                }
            }
            return;
        }

        // C. Right panel (Track Details, Contextual Action Buttons & Lyrics)
        if mouse.column >= left_width && mouse.row >= body_top {
            let right_panel_rect = Rect {
                x: left_width,
                y: body_top,
                width: cols.saturating_sub(left_width),
                height: rows.saturating_sub(3 + body_top),
            };

            let rects = get_inspector_button_rects(right_panel_rect);
            if is_hovered(Some((mouse.column, mouse.row)), rects.art_rect) {
                app.cycle_image_protocol();
                return;
            }
            if is_hovered(Some((mouse.column, mouse.row)), rects.play_rect) {
                app.play_current_track();
                return;
            }
            if is_hovered(Some((mouse.column, mouse.row)), rects.stop_rect) {
                app.stop_playback();
                return;
            }
            if is_hovered(Some((mouse.column, mouse.row)), rects.download_rect) {
                app.start_downloads();
                return;
            }
            if is_hovered(Some((mouse.column, mouse.row)), rects.select_rect) {
                app.toggle_select_current();
                return;
            }

            // Click inside lyrics area scrolls up/down
            let lyrics_top = body_top + 1 + 11;
            if mouse.row >= lyrics_top {
                let lyrics_height = rows.saturating_sub(3 + lyrics_top);
                let mid_lyrics = lyrics_top + lyrics_height / 2;
                if mouse.row < mid_lyrics {
                    app.scroll_details_up();
                } else {
                    app.scroll_details_down();
                }
            }
        }
    }
}

fn handle_footer_click(app: &mut App, area: Rect, col: u16, row: u16) {
    let hitboxes = get_footer_hitboxes(area);
    let pos = Some((col, row));

    if is_hovered(pos, hitboxes.quit_rect) {
        app.should_quit = true;
    } else if is_hovered(pos, hitboxes.add_rect) {
        app.url_input.clear();
        app.active_modal = ActiveModal::AddUrl;
    } else if is_hovered(pos, hitboxes.search_rect) {
        app.search_mode = !app.search_mode;
    } else if is_hovered(pos, hitboxes.refresh_rect) {
        app.refresh_library();
    } else if is_hovered(pos, hitboxes.settings_rect) {
        app.open_settings();
    } else if is_hovered(pos, hitboxes.help_rect) {
        app.active_modal = ActiveModal::Help;
    }
}

fn handle_add_url_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            let input = app.url_input.trim().to_string();
            if !input.is_empty() {
                app.fetch_single_clip(input);
            }
            app.active_modal = ActiveModal::None;
        }
        KeyCode::Esc => {
            app.active_modal = ActiveModal::None;
        }
        KeyCode::Backspace => {
            app.url_input.pop();
        }
        KeyCode::Char(c) => {
            app.url_input.push(c);
        }
        _ => {}
    }
}

fn handle_settings_keys(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.active_modal = ActiveModal::None;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.settings_field = app.settings_field.next();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.settings_field = app.settings_field.prev();
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.save_settings();
        }
        KeyCode::Char(c)
            if key.modifiers.contains(KeyModifiers::CONTROL) && normalize_cyrillic(c) == 's' =>
        {
            app.save_settings();
        }
        KeyCode::Char('t')
            if app.settings_field != SettingsField::Token
                && app.settings_field != SettingsField::DownloadDir
                && app.settings_field != SettingsField::FileTemplate =>
        {
            app.try_autodetect_browser_token();
        }
        KeyCode::Char(c)
            if (app.settings_field != SettingsField::Token
                && app.settings_field != SettingsField::DownloadDir
                && app.settings_field != SettingsField::FileTemplate)
                && normalize_cyrillic(c) == 't' =>
        {
            app.try_autodetect_browser_token();
        }
        KeyCode::F(2) => {
            app.try_autodetect_browser_token();
        }
        KeyCode::Enter => match app.settings_field {
            SettingsField::AutoDetectBtn => app.try_autodetect_browser_token(),
            SettingsField::GraphicsProtocol => app.cycle_image_protocol(),
            SettingsField::EmbedId3 => app.settings_embed_id3 = !app.settings_embed_id3,
            SettingsField::SaveCover => app.settings_save_cover = !app.settings_save_cover,
            SettingsField::SaveLyrics => app.settings_save_lyrics = !app.settings_save_lyrics,
            SettingsField::CancelBtn => app.active_modal = ActiveModal::None,
            _ => app.save_settings(),
        },
        KeyCode::Char(' ') => match app.settings_field {
            SettingsField::GraphicsProtocol => app.cycle_image_protocol(),
            SettingsField::EmbedId3 => app.settings_embed_id3 = !app.settings_embed_id3,
            SettingsField::SaveCover => app.settings_save_cover = !app.settings_save_cover,
            SettingsField::SaveLyrics => app.settings_save_lyrics = !app.settings_save_lyrics,
            SettingsField::AutoDetectBtn => app.try_autodetect_browser_token(),
            SettingsField::Concurrency => {
                app.settings_concurrency = if app.settings_concurrency >= 10 {
                    1
                } else {
                    app.settings_concurrency + 1
                };
            }
            SettingsField::Token => app.settings_token_input.push(' '),
            SettingsField::DownloadDir => app.settings_download_dir_input.push(' '),
            SettingsField::FileTemplate => app.settings_file_template_input.push(' '),
            SettingsField::SaveBtn => app.save_settings(),
            SettingsField::CancelBtn => app.active_modal = ActiveModal::None,
        },
        KeyCode::Left => match app.settings_field {
            SettingsField::Concurrency if app.settings_concurrency > 1 => {
                app.settings_concurrency -= 1;
            }
            SettingsField::GraphicsProtocol => {
                app.cycle_image_protocol();
            }
            _ => {}
        },
        KeyCode::Right => match app.settings_field {
            SettingsField::Concurrency if app.settings_concurrency < 10 => {
                app.settings_concurrency += 1;
            }
            SettingsField::GraphicsProtocol => {
                app.cycle_image_protocol();
            }
            _ => {}
        },
        KeyCode::Backspace => match app.settings_field {
            SettingsField::Token => {
                app.settings_token_input.pop();
            }
            SettingsField::DownloadDir => {
                app.settings_download_dir_input.pop();
            }
            SettingsField::FileTemplate => {
                app.settings_file_template_input.pop();
            }
            _ => {}
        },
        KeyCode::Char(c) => match app.settings_field {
            SettingsField::Token => app.settings_token_input.push(c),
            SettingsField::DownloadDir => app.settings_download_dir_input.push(c),
            SettingsField::FileTemplate => app.settings_file_template_input.push(c),
            SettingsField::GraphicsProtocol => {
                let norm = normalize_cyrillic(c);
                if norm == ' ' || norm == 'x' || norm == 'o' || norm == 'i' {
                    app.cycle_image_protocol();
                } else if norm == 'q' {
                    app.active_modal = ActiveModal::None;
                }
            }
            SettingsField::EmbedId3 => {
                let norm = normalize_cyrillic(c);
                if norm == ' ' || norm == 'x' || norm == 'o' {
                    app.settings_embed_id3 = !app.settings_embed_id3;
                } else if norm == 'q' {
                    app.active_modal = ActiveModal::None;
                }
            }
            SettingsField::SaveCover => {
                let norm = normalize_cyrillic(c);
                if norm == ' ' || norm == 'x' || norm == 'o' {
                    app.settings_save_cover = !app.settings_save_cover;
                } else if norm == 'q' {
                    app.active_modal = ActiveModal::None;
                }
            }
            SettingsField::SaveLyrics => {
                let norm = normalize_cyrillic(c);
                if norm == ' ' || norm == 'x' || norm == 'o' {
                    app.settings_save_lyrics = !app.settings_save_lyrics;
                } else if norm == 'q' {
                    app.active_modal = ActiveModal::None;
                }
            }
            _ => {}
        },
        _ => {}
    }
}

/// Normalizes Cyrillic Russian keyboard layout keypresses to their QWERTY equivalents
/// so that hotkeys work reliably when Russian keyboard layout is active.
pub fn normalize_cyrillic(c: char) -> char {
    match c {
        'й' => 'q',
        'ц' => 'w',
        'у' => 'e',
        'к' => 'r',
        'е' => 't',
        'н' => 'y',
        'г' => 'u',
        'ш' => 'i',
        'щ' => 'o',
        'з' => 'p',
        'х' => '[',
        'ъ' => ']',
        'ф' => 'a',
        'ы' => 's',
        'в' => 'd',
        'а' => 'f',
        'п' => 'g',
        'р' => 'h',
        'о' => 'j',
        'л' => 'k',
        'д' => 'l',
        'ж' => ';',
        'э' => '\'',
        'я' => 'z',
        'ч' => 'x',
        'с' => 'c',
        'м' => 'v',
        'и' => 'b',
        'т' => 'n',
        'ь' => 'm',
        'б' => '<',
        'ю' => '>',
        'Й' => 'Q',
        'Ц' => 'W',
        'У' => 'E',
        'К' => 'R',
        'Е' => 'T',
        'Н' => 'Y',
        'Г' => 'U',
        'Ш' => 'I',
        'Щ' => 'O',
        'З' => 'P',
        'Х' => '{',
        'Ъ' => '}',
        'Ф' => 'A',
        'Ы' => 'S',
        'В' => 'D',
        'А' => 'F',
        'П' => 'G',
        'Р' => 'H',
        'О' => 'J',
        'Л' => 'K',
        'Д' => 'L',
        'Ж' => ':',
        'Э' => '"',
        'Я' => 'Z',
        'Ч' => 'X',
        'С' => 'C',
        'М' => 'V',
        'И' => 'B',
        'Т' => 'N',
        'Ь' => 'M',
        'Б' => '<',
        'Ю' => '>',
        '.' => '/', // Russian slash position
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cyrillic() {
        assert_eq!(normalize_cyrillic('й'), 'q');
        assert_eq!(normalize_cyrillic('в'), 'd');
        assert_eq!(normalize_cyrillic('з'), 'p');
        assert_eq!(normalize_cyrillic('ы'), 's');
        assert_eq!(normalize_cyrillic('ф'), 'a');
        assert_eq!(normalize_cyrillic('к'), 'r');
        assert_eq!(normalize_cyrillic('с'), 'c');
        assert_eq!(normalize_cyrillic('О'), 'J');
        assert_eq!(normalize_cyrillic('Л'), 'K');
        assert_eq!(normalize_cyrillic('.'), '/');
        assert_eq!(normalize_cyrillic('q'), 'q');
    }
}
