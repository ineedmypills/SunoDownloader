use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerBackend {
    Mpv,
    Ffplay,
    None,
}

impl PlayerBackend {
    pub fn name(&self) -> &'static str {
        match self {
            PlayerBackend::Mpv => "mpv",
            PlayerBackend::Ffplay => "ffplay",
            PlayerBackend::None => "none",
        }
    }
}

pub struct AudioPlayer {
    backend: PlayerBackend,
    child: Option<Child>,
    current_title: Option<String>,
    current_id: Option<String>,
    is_paused: bool,
    ipc_path: Option<PathBuf>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let backend = if which("mpv") {
            PlayerBackend::Mpv
        } else if which("ffplay") {
            PlayerBackend::Ffplay
        } else {
            PlayerBackend::None
        };

        Self {
            backend,
            child: None,
            current_title: None,
            current_id: None,
            is_paused: false,
            ipc_path: None,
        }
    }

    pub fn backend(&self) -> PlayerBackend {
        self.backend
    }

    pub fn play(&mut self, id: String, title: String, url_or_path: &str) {
        self.stop();

        let mut ipc_path = None;
        let child = match self.backend {
            PlayerBackend::Mpv => {
                let sock =
                    std::env::temp_dir().join(format!("suno-mpv-{}.sock", std::process::id()));
                let _ = std::fs::remove_file(&sock);
                let sock_arg = format!("--input-ipc-server={}", sock.to_string_lossy());
                ipc_path = Some(sock);

                Command::new("mpv")
                    .arg("--no-video")
                    .arg("--really-quiet")
                    .arg("--terminal=no")
                    .arg(sock_arg)
                    .arg(url_or_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .ok()
            }
            PlayerBackend::Ffplay => Command::new("ffplay")
                .arg("-nodisp")
                .arg("-autoexit")
                .arg("-loglevel")
                .arg("quiet")
                .arg(url_or_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok(),
            PlayerBackend::None => None,
        };

        if child.is_some() {
            self.child = child;
            self.current_title = Some(title);
            self.current_id = Some(id);
            self.is_paused = false;
            self.ipc_path = ipc_path;
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.child.is_some() {
            #[cfg(unix)]
            if matches!(self.backend, PlayerBackend::Mpv) {
                if let Some(sock) = &self.ipc_path {
                    if let Ok(mut stream) = UnixStream::connect(sock) {
                        let _ = stream.write_all(b"{\"command\": [\"cycle\", \"pause\"]}\n");
                        let _ = stream.flush();
                    }
                }
            }
            self.is_paused = !self.is_paused;
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        #[cfg(unix)]
        if let Some(sock) = self.ipc_path.take() {
            let _ = std::fs::remove_file(sock);
        }
        self.current_title = None;
        self.current_id = None;
        self.is_paused = false;
    }

    pub fn is_playing(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    self.current_title = None;
                    self.current_id = None;
                    self.is_paused = false;
                    #[cfg(unix)]
                    if let Some(sock) = self.ipc_path.take() {
                        let _ = std::fs::remove_file(sock);
                    }
                    false
                }
                Ok(None) => !self.is_paused,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn is_active(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    self.current_title = None;
                    self.current_id = None;
                    self.is_paused = false;
                    #[cfg(unix)]
                    if let Some(sock) = self.ipc_path.take() {
                        let _ = std::fs::remove_file(sock);
                    }
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn current_title(&self) -> Option<&str> {
        self.current_title.as_deref()
    }

    pub fn current_id(&self) -> Option<&str> {
        self.current_id.as_deref()
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn which(cmd: &str) -> bool {
    // 1. Search PATH directories in pure Rust (works seamlessly on Windows, macOS, and Linux)
    if let Some(path_var) = std::env::var_os("PATH") {
        #[cfg(windows)]
        let extensions: &[&str] = &[".exe", ".cmd", ".bat", ""];
        #[cfg(not(windows))]
        let extensions: &[&str] = &[""];

        for dir in std::env::split_paths(&path_var) {
            for ext in extensions {
                let candidate = dir.join(format!("{}{}", cmd, ext));
                if candidate.is_file() {
                    return true;
                }
            }
        }
    }

    // 2. On Unix-like systems (Linux, macOS, BSD), also try 'which' command as fallback
    #[cfg(unix)]
    {
        if Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}
