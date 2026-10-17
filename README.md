# 🎵 Suno.ai Downloader TUI

[![CI](https://github.com/pilldev/suno-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/pilldev/suno-tui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/pilldev/suno-tui?color=green)](https://github.com/pilldev/suno-tui/releases)
[![AUR version](https://img.shields.io/aur/version/suno-tui.svg?color=blue)](https://aur.archlinux.org/packages/suno-tui)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

**English** | [Русский](README.ru.md)

A blazing-fast, beautiful terminal user interface (TUI) and CLI client written in **Rust** for exploring, previewing, and downloading tracks from **Suno.ai** directly to your local music library.

---

## ✨ Features

- 🖥 **Modern Two-Panel TUI Layout**:
  - **Left Panel**: Interactive track list with instant fuzzy search (`/`), pagination, live download status indicators, and batch selection (`[Space]`, `Ctrl+A`, `Ctrl+D`).
  - **Right Panel**: Detailed inspector with scrollable lyrics & prompt viewer, style tags, model version badges, duration, and track ID.
- 🖼 **Native Terminal Graphics & Cover Art**:
  - **Sixel Graphics**: Crystal-clear album artwork in KDE Konsole, Foot, and WezTerm.
  - **Kitty Protocol**: Supported in native Kitty and Ghostty.
  - **Unicode Halfblocks**: Universal TrueColor fallback for all other terminals.
  - Easy runtime graphics switching directly in Settings (`[c]`) or via `[i]`.
- 🎧 **In-TUI Audio Streaming & Preview**:
  - Instant audio stream playback powered by `mpv` without leaving the terminal.
  - Positive UI approach with non-blocking stream buffer handling.
- 🖱 **Complete Mouse & Keyboard Interactivity**:
  - Click to select, inspect, play, or toggle downloads.
  - Mouse wheel support for track list and lyrics inspector.
  - Contextual buttons and interactive status pills.
  - **Full Cyrillic Layout Support**: All hotkeys work seamlessly regardless of whether your system layout is English or Russian (йцукен).
- 🏷 **Universal ID3v2.3 Metadata Embedding**:
  - Track Title (`TIT2`), Artist (`TPE1`), Album (`TALB`)
  - Musical style & genre tags (`TCON`)
  - Full Lyrics & generation prompt (`USLT`)
  - Front Cover Artwork image (`APIC`)
  - 320 kbps (48 kHz Stereo) MP3 conversion via FFmpeg.
- 🔐 **Zero-Effort Session Decryption**:
  - Automatically reads and decrypts Suno session tokens from Chrome, Chrome Unstable, Chromium, Brave, and Edge on Linux using KDE KWallet and GNOME Keyring.
- 🤖 **Headless CLI Mode**:
  - Direct command-line downloading without launching the TUI interface, perfect for scripts and automated workflows.

---

## 📦 Installation

### Arch Linux (AUR)

`suno-tui` is available in the Arch User Repository:

```bash
# Using paru (Recommended):
paru -S suno-tui
# Or pre-compiled binary:
paru -S suno-tui-bin

# Using yay:
yay -S suno-tui
# Or pre-compiled binary:
yay -S suno-tui-bin
```

### Pre-built Binaries (Linux, macOS, Windows)

Download the latest pre-compiled release archive for your operating system and architecture from the [GitHub Releases](https://github.com/pilldev/suno-tui/releases) page:

- `x86_64-unknown-linux-gnu.tar.gz` (Linux x86_64)
- `x86_64-unknown-linux-musl.tar.gz` (Linux x86_64 Static)
- `aarch64-unknown-linux-gnu.tar.gz` (Linux ARM64)
- `x86_64-apple-darwin.tar.gz` (macOS Intel)
- `aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)
- `x86_64-pc-windows-msvc.zip` (Windows x64)

Extract the archive and move the binary to your `PATH` (e.g. `/usr/local/bin` on Linux/macOS).

### Build from Source (Cargo)

Requires **Rust 1.80+** (`cargo`):

```bash
git clone https://github.com/pilldev/suno-tui.git
cd suno-tui
cargo build --release
```

The optimized executable will be located at `target/release/suno-tui`.

---

## 🛠 System Dependencies

- **`mpv`** *(Optional, recommended)*: Enables in-TUI audio streaming and preview.
  ```bash
  # Arch / Manjaro
  sudo pacman -S mpv
  # Ubuntu / Debian
  sudo apt install mpv
  # Fedora
  sudo dnf install mpv
  # macOS
  brew install mpv
  ```
- **`ffmpeg`** *(Optional, recommended)*: Enables high-fidelity 320 kbps MP3 transcoding and ID3 tagging.
  ```bash
  sudo pacman -S ffmpeg   # Arch
  sudo apt install ffmpeg # Ubuntu / Debian
  brew install ffmpeg     # macOS
  ```

---

## 🔑 Account & Library Configuration

To browse and export your personal creations, connect your account using one of these easy methods:

### Option A: Automatic Browser Session Decryption (Easiest on Linux)
`suno-tui` automatically detects active Suno sessions in **Google Chrome, Chrome Unstable, Brave, Chromium, and Edge**, decrypting credentials securely via **KDE KWallet** or **GNOME Keyring/Secret Service**. Simply launch `suno-tui` or press **`t`** inside Settings!

### Option B: One-Click Bookmarklet
1. In your web browser, create a bookmark with the following URL:
   ```javascript
   javascript:(function(){const m=document.cookie.match(/__session=([^;]+)/);if(m){navigator.clipboard.writeText(m[1]);alert("✓ Suno Token copied to clipboard!");}else{alert("Log in to suno.com first");}})();
   ```
2. Open [suno.com](https://suno.com) and click the bookmark.
3. In `suno-tui`, press **`c`** (or click `[⚙ Config]`), paste the token, and press **Enter** or **Ctrl+S**.

### Option C: DevTools (Manual)
1. Open [suno.com](https://suno.com), press **F12**, switch to the **Network** tab, and filter by `studio-api`.
2. Inspect any request and copy the `Authorization: Bearer <TOKEN>` header value or `__session` cookie.
3. In `suno-tui`, press **`c`**, paste and save. Press **`r`** to sync your library!

---

## ⌨️ Controls & Shortcuts

| Action | Mouse | Keyboard |
|---|---|---|
| Select / Navigate track | **Left Click** on track | `↑` / `k`, `↓` / `j`, `Home` / `End` |
| Play audio stream | **Click track** or `[▶ Play]` | `p` |
| Stop audio stream | **Click** `[⏹ Stop]` | `s` |
| Toggle cover graphics | **Click** Art Box | `i` (or configure in Settings) |
| Scroll list / lyrics | **Mouse Wheel** | `PgUp` / `PgDown`, `J` / `K` |
| Toggle track selection | **Click** `[☑]` checkbox | `Space` |
| Select / Deselect all | — | `Ctrl + A` / `Ctrl + D` |
| Download track(s) | **Click** `[⬇ Download]` | `d` |
| Add public URL / UUID | **Click** `[+ Add Track]` | `a` or `u` |
| Filter & Search | **Click** `[🔍 Search]` | `/` (`Esc` to clear) |
| Refresh library | **Click** `[↻ Refresh]` | `r` |
| Settings & Graphics | **Click** `[⚙ Config]` | `c` or `S` |
| Save settings | **Click** `[✓ Save]` | `Ctrl + S` or `Enter` |
| Auto-detect Browser Token | **Click** `[⚡ Auto-Detect]` | `t` or `F2` |
| Help | **Click** `[? Help]` | `h` or `?` |
| Quit | **Click** `[✕ Quit]` | `q` or `Esc` |

*(Note: Russian Cyrillic keyboard layouts are fully supported without switching layouts)*

---

## 💻 Headless CLI Usage

Download tracks directly without launching the graphical interface:

```bash
# Download a public track by URL:
suno-tui --cli "https://suno.com/song/01f720ca-2f95-45db-ad04-9ceb9bc52e3e"

# Download to a specific directory by UUID:
suno-tui --cli -o ~/Music 01f720ca-2f95-45db-ad04-9ceb9bc52e3e

# Launch TUI with pre-loaded track links:
suno-tui "https://suno.com/song/01f720ca-2f95-45db-ad04-9ceb9bc52e3e"
```

---

## 📁 Configuration File

Settings are saved automatically to `~/.config/suno-downloader/config.toml`:

```toml
session_token = "ey..."
download_dir = "/home/user/Music/Suno"
file_template = "{title} ({id})"
embed_id3 = true
save_cover_image = false
save_lyrics_file = false
concurrency = 3
```

---

## 📄 License

Distributed under the [MIT License](LICENSE).
