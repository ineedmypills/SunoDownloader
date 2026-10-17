# 🎵 Suno.ai Downloader TUI

[![CI](https://github.com/pilldev/suno-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/pilldev/suno-tui/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/pilldev/suno-tui?color=green)](https://github.com/pilldev/suno-tui/releases)
[![AUR version](https://img.shields.io/aur/version/suno-tui.svg?color=blue)](https://aur.archlinux.org/packages/suno-tui)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

[English](README.md) | **Русский**

Быстрый, эстетичный и функциональный консольный TUI (Terminal User Interface) и CLI клиент на **Rust** для поиска, воспроизведения и скачивания треков из **Suno.ai** напрямую в вашу медиатеку.

---

## ✨ Основные возможности

- 🖥 **Современный двухпанельный TUI интерфейс**:
  - **Слева**: список треков с быстрым поиском (`/`), пагинацией, живыми индикаторами статуса и чекбоксами для пакетного выбора (`[Space]`, `Ctrl+A`, `Ctrl+D`).
  - **Справа**: детальная панель информации с текстом песни (Lyrics / Prompt), музыкальным стилем, бейджами версий модели, длительностью и UUID.
- 🖼 **Нативная терминальная графика и обложки**:
  - **Sixel Graphics**: четкие обложки альбомов в высоком разрешении в KDE Konsole, Foot, WezTerm.
  - **Kitty Protocol**: поддержка в нативных Kitty и Ghostty.
  - **Unicode Halfblocks**: универсальный TrueColor-режим для всех остальных терминалов.
  - Удобное переключение протоколов прямо в окне настроек (`[c]`) или клавишей `[i]`.
- 🎧 **Встроенный аудиоплеер прямо в TUI**:
  - Мгновенное онлайн-воспроизведение аудиопотока через `mpv` без выхода из терминала.
  - Полностью positive UI подход без подвисания интерфейса.
- 🖱 **Полноценное управление мышью и клавиатурой**:
  - Клик левой кнопкой мыши по трекам, чекбоксам и контекстным кнопкам управления.
  - Прокрутка колесом мыши списка песен и текста в правой панели.
  - **Полная поддержка русской раскладки**: все горячие клавиши работают одинаково независимо от активного языка системы (йцукен / QWERTY).
- 🏷 **Универсальные метаданные ID3v2.3**:
  - Название трека (`TIT2`), автор (`TPE1`), альбом (`TALB`).
  - Жанр и музыкальный стиль (`TCON`).
  - Полный текст песни и промпт (`USLT`).
  - Вшитая фронтальная обложка альбома (`APIC`).
  - Автоматическая транскодировка в MP3 320 kbps (48 kHz Stereo) через FFmpeg.
- 🔐 **Автоматическое извлечение сессии из браузера**:
  - Безопасное сканирование и расшифровка сессии Suno из Google Chrome, Chrome Unstable, Chromium, Brave и Edge на Linux через **KDE KWallet** и **GNOME Keyring/Secret Service**.
- 🤖 **Режим CLI (Headless)**:
  - Прямое скачивание из терминала или скриптов без запуска графического интерфейса.

---

## 📦 Установка

### Arch Linux (AUR)

Пакеты доступны в Arch User Repository:

```bash
# Через paru (рекомендуется):
paru -S suno-tui
# Или готовый бинарный пакет:
paru -S suno-tui-bin

# Через yay:
yay -S suno-tui
# Или готовый бинарный пакет:
yay -S suno-tui-bin
```

### Готовые бинарные файлы (Linux, macOS, Windows)

Скачайте релизный архив для вашей платформы со страницы [GitHub Releases](https://github.com/pilldev/suno-tui/releases):

- `x86_64-unknown-linux-gnu.tar.gz` (Linux x86_64)
- `x86_64-unknown-linux-musl.tar.gz` (Linux x86_64 Static)
- `aarch64-unknown-linux-gnu.tar.gz` (Linux ARM64)
- `x86_64-apple-darwin.tar.gz` (macOS Intel)
- `aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)
- `x86_64-pc-windows-msvc.zip` (Windows x64)

Распакуйте архив и поместите бинарный файл в системный `PATH` (например, `/usr/local/bin`).

### Сборка из исходников (Cargo)

Требуется компилятор **Rust 1.80+** (`cargo`):

```bash
git clone https://github.com/pilldev/suno-tui.git
cd suno-tui
cargo build --release
```

Готовый бинарный файл будет доступен в: `target/release/suno-tui`.

---

## 🛠 Системные зависимости

- **`mpv`** *(Рекомендуется)*: для потокового воспроизведения аудио прямо в терминале.
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
- **`ffmpeg`** *(Рекомендуется)*: для конвертации в 320 kbps MP3 и вшивания тегов ID3v2.3.
  ```bash
  sudo pacman -S ffmpeg   # Arch
  sudo apt install ffmpeg # Ubuntu / Debian
  brew install ffmpeg     # macOS
  ```

---

## 🔑 Подключение личной библиотеки

### Вариант 1: Полностью автоматическое извлечение из браузера (Linux)
`suno-tui` автоматически определяет активную сессию Suno в **Chrome, Chrome Unstable, Brave, Chromium и Edge**, расшифровывая сессионный токен через **KDE KWallet** или **GNOME Keyring**. Просто запустите программу или нажмите клавишу **`t`** в настройках!

### Вариант 2: Букмарклет в один клик
1. В браузере создайте закладку с таким адресом (URL):
   ```javascript
   javascript:(function(){const m=document.cookie.match(/__session=([^;]+)/);if(m){navigator.clipboard.writeText(m[1]);alert("✓ Токен Suno скопирован в буфер обмена!");}else{alert("Сначала авторизуйтесь на suno.com");}})();
   ```
2. Откройте [suno.com](https://suno.com) и нажмите на эту закладку.
3. В `suno-tui` нажмите **`c`** (или кликните по `[⚙ Config]`), вставьте токен и нажмите **Enter** или **Ctrl+S**.

### Вариант 3: Через DevTools (F12)
1. На [suno.com](https://suno.com) нажмите **F12** -> вкладка **Network** -> фильтр `studio-api`.
2. Скопируйте заголовок `Authorization: Bearer <TOKEN>` или куки `__session=...`.
3. В `suno-tui` нажмите **`c`**, вставьте и сохраните. Нажмите **`r`** для синхронизации!

---

## ⌨️ Управление (Мышь и Клавиатура)

| Действие | Мышь | Клавиатура |
|---|---|---|
| Выбор трека в списке | **Клик ЛКМ** по строке | `↑` / `k`, `↓` / `j`, `Home` / `End` |
| Воспроизведение трека | **Клик по треку** или `[▶ Play]` | `p` (з) |
| Остановка плеера | **Клик** `[⏹ Stop]` | `s` (ы) |
| Переключение протокола графики | **Клик** по блоку обложки | `i` (ш) или в меню настроек |
| Прокрутка списка / текста | **Колесо мыши** | `PgUp` / `PgDown`, `J` / `K` |
| Чекбокс мультивыбора | **Клик** по `[☑]` | `Space` (Пробел) |
| Выбрать / снять все | — | `Ctrl + A` (Ctrl+Ф) / `Ctrl + D` (Ctrl+В) |
| Скачать выбранные | **Клик** `[⬇ Download]` | `d` (в) |
| Добавить ссылку / UUID | **Клик** `[+ Add Track]` | `a` (ф) или `u` (г) |
| Поиск и фильтр | **Клик** `[🔍 Search]` | `/` (`.` на рус. раскладке, `Esc` — сброс) |
| Обновить библиотеку | **Клик** `[↻ Refresh]` | `r` (к) |
| Настройки и протоколы | **Клик** `[⚙ Config]` | `c` (с) или `S` |
| Сохранить настройки | **Клик** `[✓ Save]` | `Ctrl + S` (Ctrl+Ы) или `Enter` |
| Авто-поиск сессии | **Клик** `[⚡ Auto-Detect]` | `t` (е) или `F2` |
| Справка | **Клик** `[? Help]` | `h` (р) или `?` |
| Выход | **Клик** `[✕ Quit]` | `q` (й) или `Esc` |

*(Русская раскладка клавиатуры полностью поддерживается без необходимости переключения)*

---

## 💻 Использование через консоль (CLI)

```bash
# Скачивание публичного трека по URL:
suno-tui --cli "https://suno.com/song/01f720ca-2f95-45db-ad04-9ceb9bc52e3e"

# Скачивание в указанную директорию по UUID:
suno-tui --cli -o ~/Music 01f720ca-2f95-45db-ad04-9ceb9bc52e3e

# Запуск TUI с предварительно загруженными ссылками:
suno-tui "https://suno.com/song/01f720ca-2f95-45db-ad04-9ceb9bc52e3e"
```

---

## 📁 Файл конфигурации

Настройки сохраняются автоматически в `~/.config/suno-downloader/config.toml`:

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

## 📄 Лицензия

Распространяется под лицензией [MIT](LICENSE).
