use aes::cipher::block_padding::NoPadding;
use aes::cipher::{BlockModeDecrypt, KeyIvInit};
use aes::Aes128;
use anyhow::Result;
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Aes128CbcDec = cbc::Decryptor<Aes128>;

pub struct BrowserTokenDetector;

impl BrowserTokenDetector {
    /// Attempt to find and decrypt Suno session token from all installed browsers
    pub fn detect() -> Option<String> {
        let home = dirs::home_dir()?;

        // 1. Gather all candidate Chromium cookie database paths
        let mut chromium_paths = Vec::new();

        let chrome_base_names = [
            "google-chrome-unstable",
            "google-chrome",
            "google-chrome-beta",
            "chromium",
            "BraveSoftware/Brave-Browser",
            "microsoft-edge",
            "opera",
            "vivaldi",
            "Google/Chrome",
            "Google/Chrome Canary",
            "BraveSoftware/Brave-Browser",
            "Microsoft/Edge",
        ];

        let profiles = ["Default", "Profile 1", "Profile 2", "Profile 3"];

        #[allow(unused_mut)]
        let mut config_roots = vec![home.join(".config")];
        #[cfg(target_os = "macos")]
        config_roots.push(home.join("Library/Application Support"));
        #[cfg(target_os = "windows")]
        if let Some(local_appdata) = dirs::data_local_dir() {
            config_roots.push(local_appdata);
        }

        for root in config_roots {
            for base in chrome_base_names {
                let base_dir = root.join(base);
                let resolved_base = fs::canonicalize(&base_dir).unwrap_or(base_dir);

                for profile in profiles {
                    let p1 = resolved_base.join(profile).join("Cookies");
                    let p2 = resolved_base.join(profile).join("Network").join("Cookies");
                    if p1.exists() {
                        chromium_paths.push((base.to_string(), p1));
                    }
                    if p2.exists() {
                        chromium_paths.push((base.to_string(), p2));
                    }
                }
            }
        }

        // Also check psd /run/user/1000/psd/ locations directly
        if let Ok(run_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let psd_dir = PathBuf::from(run_dir).join("psd");
            if psd_dir.exists() {
                if let Ok(entries) = fs::read_dir(&psd_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        for profile in profiles {
                            let p1 = path.join(profile).join("Cookies");
                            let p2 = path.join(profile).join("Network").join("Cookies");
                            if p1.exists() {
                                chromium_paths.push(("chrome-psd".to_string(), p1));
                            }
                            if p2.exists() {
                                chromium_paths.push(("chrome-psd".to_string(), p2));
                            }
                        }
                    }
                }
            }
        }

        // Try extracting from each Chromium candidate
        for (browser_type, db_path) in &chromium_paths {
            if let Some(token) = Self::extract_from_chromium(browser_type, db_path) {
                return Some(token);
            }
        }

        // 2. Check Firefox cookie databases (unencrypted in SQLite)
        #[allow(unused_mut)]
        let mut firefox_dirs = vec![home.join(".mozilla/firefox")];
        #[cfg(target_os = "macos")]
        firefox_dirs.push(home.join("Library/Application Support/Firefox/Profiles"));
        #[cfg(target_os = "windows")]
        if let Some(roaming) = dirs::config_dir() {
            firefox_dirs.push(roaming.join("Mozilla/Firefox/Profiles"));
        }

        for firefox_dir in firefox_dirs {
            if firefox_dir.exists() {
                if let Ok(entries) = fs::read_dir(&firefox_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path().join("cookies.sqlite");
                        if path.exists() {
                            if let Some(token) = Self::extract_from_firefox(&path) {
                                return Some(token);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_from_chromium(browser_type: &str, db_path: &Path) -> Option<String> {
        // Copy database to a temporary location to avoid database locks while browser is open
        let temp_dir = std::env::temp_dir();
        let rand_id: u32 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(12345);
        let temp_db = temp_dir.join(format!("suno_cookies_{}.sqlite", rand_id));

        if fs::copy(db_path, &temp_db).is_err() {
            return None;
        }

        // Query both value and hex(encrypted_value)
        let query = "SELECT name, value, hex(encrypted_value) FROM cookies WHERE host_key LIKE '%suno%' AND name IN ('__session', '__session_Jnxw-muT');";
        let output = Command::new("sqlite3")
            .arg(&temp_db)
            .arg(query)
            .output()
            .ok();

        // Clean up temp db
        let _ = fs::remove_file(&temp_db);

        let output = output?;
        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Passwords to try for decryption
        let passwords = Self::get_browser_passwords(browser_type);

        for line in lines {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                let clear_val = parts[1].trim();
                let hex_enc = parts[2].trim();

                // If clear value already contains a JWT or valid session
                if !clear_val.is_empty() && (clear_val.starts_with("ey") || clear_val.len() > 30) {
                    return Some(clear_val.to_string());
                }

                // If encrypted, decrypt it
                if !hex_enc.is_empty() {
                    if let Ok(raw_bytes) = hex_to_bytes(hex_enc) {
                        for pwd in &passwords {
                            if let Some(token) = Self::decrypt_chromium_cookie(&raw_bytes, pwd) {
                                return Some(token);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_from_firefox(db_path: &Path) -> Option<String> {
        let temp_dir = std::env::temp_dir();
        let rand_id: u32 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(54321);
        let temp_db = temp_dir.join(format!("suno_moz_cookies_{}.sqlite", rand_id));

        if fs::copy(db_path, &temp_db).is_err() {
            return None;
        }

        let query =
            "SELECT value FROM moz_cookies WHERE host LIKE '%suno%' AND name = '__session';";
        let output = Command::new("sqlite3")
            .arg(&temp_db)
            .arg(query)
            .output()
            .ok();

        let _ = fs::remove_file(&temp_db);

        let output = output?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() && (text.starts_with("ey") || text.len() > 30) {
                return Some(text);
            }
        }

        None
    }

    /// Retrieve browser safe storage passwords from KWallet, Secret Service, or defaults
    fn get_browser_passwords(browser_type: &str) -> Vec<String> {
        let mut passwords = Vec::new();

        // 1. KWallet (KDE Plasma) - check kwalletd6 and kwalletd5
        for daemon in &["org.kde.kwalletd6", "org.kde.kwalletd5"] {
            let mod_path = if daemon.ends_with('6') {
                "/modules/kwalletd6"
            } else {
                "/modules/kwalletd5"
            };

            let net_wallet = Command::new("qdbus")
                .args([daemon, mod_path, "org.kde.KWallet.networkWallet"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if !net_wallet.is_empty() {
                let handle_out = Command::new("qdbus")
                    .args([
                        daemon,
                        mod_path,
                        "org.kde.KWallet.open",
                        &net_wallet,
                        "0",
                        "suno-tui",
                    ])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                if let Ok(handle) = handle_out.parse::<i32>() {
                    if handle >= 0 {
                        let folders = ["Chrome Keys", "Chromium Keys"];
                        let entries = [
                            "Chrome Safe Storage",
                            "Chromium Safe Storage",
                            "Google Chrome Safe Storage",
                        ];

                        for folder in folders {
                            for entry in entries {
                                if let Ok(pwd_out) = Command::new("qdbus")
                                    .args([
                                        daemon,
                                        mod_path,
                                        "org.kde.KWallet.readPassword",
                                        &handle.to_string(),
                                        folder,
                                        entry,
                                        "suno-tui",
                                    ])
                                    .output()
                                {
                                    if pwd_out.status.success() {
                                        let pwd = String::from_utf8_lossy(&pwd_out.stdout)
                                            .trim()
                                            .to_string();
                                        if !pwd.is_empty() && !passwords.contains(&pwd) {
                                            passwords.push(pwd);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. macOS Keychain
        #[cfg(target_os = "macos")]
        {
            let keychain_apps = [
                "Chrome Safe Storage",
                "Chromium Safe Storage",
                "Brave Safe Storage",
                "Microsoft Edge Safe Storage",
            ];
            for app in keychain_apps {
                if let Ok(out) = Command::new("security")
                    .args(["find-generic-password", "-w", "-s", app])
                    .output()
                {
                    if out.status.success() {
                        let pwd = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !pwd.is_empty() && !passwords.contains(&pwd) {
                            passwords.push(pwd);
                        }
                    }
                }
            }
        }

        // 3. Secret Service / GNOME Keyring (via secret-tool) on Linux
        let secret_apps = [
            "chrome",
            "chromium",
            "google-chrome",
            "google-chrome-unstable",
            "google-chrome-beta",
            browser_type,
        ];
        for app in secret_apps {
            if let Ok(out) = Command::new("secret-tool")
                .args(["lookup", "application", app])
                .output()
            {
                if out.status.success() {
                    let pwd = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !pwd.is_empty() && !passwords.contains(&pwd) {
                        passwords.push(pwd);
                    }
                }
            }
        }

        // 4. Fallback standard Chromium password on Linux
        passwords.push("peanuts".to_string());

        passwords
    }

    /// Decrypt Chromium v10/v11 encrypted cookie payload (Linux & macOS)
    fn decrypt_chromium_cookie(raw: &[u8], password: &str) -> Option<String> {
        if raw.len() < 19 {
            return None;
        }

        let is_v10 = &raw[..3] == b"v10";
        let is_v11 = &raw[..3] == b"v11";

        if !is_v10 && !is_v11 {
            return None;
        }

        let (iv, ciphertext) = if is_v10 {
            // v10: IV is 16 spaces
            ([b' '; 16], &raw[3..])
        } else {
            // v11: IV is raw[3..19], ciphertext is raw[19..]
            let mut iv = [0u8; 16];
            iv.copy_from_slice(&raw[3..19]);
            (iv, &raw[19..])
        };

        if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
            return None;
        }

        // Try both 1 iteration (Linux standard) and 1003 iterations (macOS standard)
        for &iters in &[1, 1003] {
            let mut key = [0u8; 16];
            pbkdf2_hmac::<Sha1>(password.as_bytes(), b"saltysalt", iters, &mut key);

            let mut buf = ciphertext.to_vec();
            let cipher = Aes128CbcDec::new(&key.into(), &iv.into());
            if let Ok(decrypted) = cipher.decrypt_padded::<NoPadding>(&mut buf) {
                // Look for JWT token marker "eyJ" in decrypted buffer
                if let Some(pos) = decrypted.windows(3).position(|w| w == b"eyJ") {
                    let slice = &decrypted[pos..];
                    // Extract contiguous valid base64url characters
                    let mut end = 0;
                    while end < slice.len() {
                        let b = slice[end];
                        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' {
                            end += 1;
                        } else {
                            break;
                        }
                    }
                    if end > 30 {
                        let token = String::from_utf8_lossy(&slice[..end]).to_string();
                        return Some(token);
                    }
                }
            }
        }

        None
    }
}

fn hex_to_bytes(s: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(s.len() / 2);
    let mut chars = s.chars();
    while let (Some(a), Some(b)) = (chars.next(), chars.next()) {
        let byte = u8::from_str_radix(&format!("{}{}", a, b), 16)?;
        bytes.push(byte);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_bytes() {
        let h = "763131";
        let b = hex_to_bytes(h).unwrap();
        assert_eq!(b, b"v11");
    }

    #[test]
    fn test_browser_detect_smoke() {
        // Ensures browser detector runs safely across any system (including CI environments)
        let token = BrowserTokenDetector::detect();
        if let Some(t) = token {
            assert!(t.starts_with("ey") || t.len() > 20);
        }
    }

    #[tokio::test]
    async fn test_reqwest_cover_fetch() {
        let url = "https://cdn2.suno.ai/image_large_45e8b50c-c3b9-4c18-a40e-11b01ac1cde9.jpeg";
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
            .build()
            .unwrap();
        let resp = client.get(url).send().await.unwrap();
        assert!(resp.status().is_success());
        let bytes = resp.bytes().await.unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert!(img.width() > 0);
    }
}
