#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Icons {
    pub is_nerd: bool,
    pub music: &'static str,
    pub search: &'static str,
    pub gear: &'static str,
    pub help: &'static str,
    pub quit: &'static str,
    pub add: &'static str,
    pub download: &'static str,
    pub refresh: &'static str,
    pub check_box_checked: &'static str,
    pub check_box_unchecked: &'static str,
    pub check: &'static str,
    pub cross: &'static str,
    pub hourglass: &'static str,
    pub arrow_down: &'static str,
    pub dot: &'static str,
    pub cursor: &'static str,
    pub artist: &'static str,
    pub clock: &'static str,
    pub lightning: &'static str,
    pub tag: &'static str,
    pub lyrics: &'static str,
    pub shield_auth: &'static str,
    pub shield_anon: &'static str,
}

impl Icons {
    /// Automatically detects whether Nerd Fonts are supported in the current environment
    pub fn detect() -> Self {
        if Self::detect_nerd_font_support() {
            Self::nerd_fonts()
        } else {
            Self::unicode()
        }
    }

    pub fn nerd_fonts() -> Self {
        Self {
            is_nerd: true,
            music: "\u{f001} ",               //  Music note
            search: "\u{f002} ",              //  Search magnifier
            gear: "\u{f013} ",                //  Gear
            help: "\u{f059} ",                //  Question circle
            quit: "\u{f00d} ",                //  Times / Close
            add: "\u{f067} ",                 //  Plus / Add
            download: "\u{f019} ",            //  Download arrow
            refresh: "\u{f021} ",             //  Refresh / Sync
            check_box_checked: "\u{f14a} ",   //  Checkbox checked
            check_box_unchecked: "\u{f096} ", //  Checkbox empty
            check: "\u{f00c} ",               //  Checkmark
            cross: "\u{f00d} ",               //  Cross
            hourglass: "\u{f252} ",           //  Hourglass
            arrow_down: "\u{f063} ",          //  Arrow down
            dot: "· ",                        // Middle dot
            cursor: "\u{f054} ",              //  Chevron / Cursor
            artist: "\u{f007} ",              //  User / Artist
            clock: "\u{f017} ",               //  Clock / Duration
            lightning: "\u{f0e7} ",           //  Lightning bolt / Model
            tag: "\u{f02b} ",                 //  Tag / Genre
            lyrics: "\u{f0f6} ",              //  Document / Lyrics
            shield_auth: "\u{f132} ",         //  Shield (Authorized)
            shield_anon: "\u{f023} ",         //  Lock (Anonymous)
        }
    }

    pub fn unicode() -> Self {
        Self {
            is_nerd: false,
            music: "♫ ",
            search: "⌕ ",
            gear: "⚙ ",
            help: "? ",
            quit: "✕ ",
            add: "+ ",
            download: "↓ ",
            refresh: "↻ ",
            check_box_checked: "☑ ",
            check_box_unchecked: "☐ ",
            check: "✔ ",
            cross: "✖ ",
            hourglass: "⏳ ",
            arrow_down: "↓ ",
            dot: "· ",
            cursor: "▎ ",
            artist: "👤 ",
            clock: "⏱ ",
            lightning: "⚡ ",
            tag: "🏷 ",
            lyrics: "📜 ",
            shield_auth: "● ",
            shield_anon: "○ ",
        }
    }

    fn detect_nerd_font_support() -> bool {
        // 1. Explicit environment variable check
        if let Ok(val) = std::env::var("NERD_FONT").or_else(|_| std::env::var("NERDFONT")) {
            let v = val.trim().to_lowercase();
            if v == "1" || v == "true" || v == "yes" {
                return true;
            }
            if v == "0" || v == "false" || v == "no" {
                return false;
            }
        }

        // 2. Known modern terminal emulators that support Nerd Fonts
        if std::env::var("GHOSTTY_RESOURCES_DIR").is_ok()
            || std::env::var("KITTY_WINDOW_ID").is_ok()
            || std::env::var("WEZTERM_PANE").is_ok()
            || std::env::var("ALACRITTY_LOG").is_ok()
            || std::env::var("WT_SESSION").is_ok()
        {
            return true;
        }

        if let Ok(prog) = std::env::var("TERM_PROGRAM") {
            let p = prog.to_lowercase();
            if p.contains("wezterm")
                || p.contains("ghostty")
                || p.contains("kitty")
                || p.contains("alacritty")
                || p.contains("warp")
                || p.contains("iterm")
                || p.contains("vscode")
            {
                return true;
            }
        }

        if let Ok(term) = std::env::var("TERM") {
            let t = term.to_lowercase();
            if t.contains("kitty")
                || t.contains("alacritty")
                || t.contains("foot")
                || t.contains("ghostty")
                || t.contains("wezterm")
            {
                return true;
            }
        }

        // 3. System font check on Linux
        if Self::has_system_nerd_fonts() {
            return true;
        }

        false
    }

    fn has_system_nerd_fonts() -> bool {
        let dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];
        for d in dirs {
            if let Ok(entries) = std::fs::read_dir(d) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.to_string_lossy();
                    if name.contains("Nerd") || name.contains("NF") {
                        return true;
                    }
                    if path.is_dir() {
                        if let Ok(sub) = std::fs::read_dir(&path) {
                            for sub_entry in sub.flatten() {
                                let sub_name = sub_entry.path().to_string_lossy().to_string();
                                if sub_name.contains("Nerd") || sub_name.contains("NF") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(home) = dirs::home_dir() {
            let user_font_dir = home.join(".local/share/fonts");
            if let Ok(entries) = std::fs::read_dir(user_font_dir) {
                for entry in entries.flatten() {
                    let name = entry.path().to_string_lossy().to_string();
                    if name.contains("Nerd") || name.contains("NF") {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_detect() {
        let icons = Icons::detect();
        assert!(!icons.music.is_empty());
        assert!(!icons.check.is_empty());
        assert!(!icons.check_box_checked.is_empty());
    }

    #[test]
    fn test_icons_nerd_and_unicode() {
        let nerd = Icons::nerd_fonts();
        assert!(nerd.is_nerd);
        assert!(!nerd.music.is_empty());
        assert_eq!(nerd.music, "\u{f001} ");

        let unicode = Icons::unicode();
        assert!(!unicode.is_nerd);
        assert!(!unicode.music.is_empty());
        assert_eq!(unicode.music, "♫ ");
    }
}
