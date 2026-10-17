use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBadge {
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBadges {
    pub songrow: Option<ModelBadge>,
    pub songcard: Option<ModelBadge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipMetadata {
    pub tags: Option<String>,
    pub prompt: Option<String>,
    pub gpt_description_prompt: Option<String>,
    pub duration: Option<f64>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    #[serde(rename = "type")]
    pub clip_type: Option<String>,
    pub task: Option<String>,
    pub model_badges: Option<ModelBadges>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaUrl {
    pub url: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub title: Option<String>,
    pub video_url: Option<String>,
    pub audio_url: Option<String>,
    pub media_urls: Option<Vec<MediaUrl>>,
    pub image_url: Option<String>,
    pub image_large_url: Option<String>,
    pub major_model_version: Option<String>,
    pub model_name: Option<String>,
    pub metadata: Option<ClipMetadata>,
    pub created_at: Option<String>,
    pub status: Option<String>,
    pub display_name: Option<String>,
    pub handle: Option<String>,
    pub is_liked: Option<bool>,
    pub is_trashed: Option<bool>,
}

impl Clip {
    pub fn display_title(&self) -> String {
        self.title
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Untitled Track")
            .to_string()
    }

    pub fn display_artist(&self) -> String {
        if let Some(name) = &self.display_name {
            if !name.trim().is_empty() {
                return name.clone();
            }
        }
        if let Some(handle) = &self.handle {
            if !handle.trim().is_empty() {
                return format!("@{}", handle);
            }
        }
        "Suno AI".to_string()
    }

    pub fn display_model(&self) -> String {
        let base_model = self
            .metadata
            .as_ref()
            .and_then(|m| m.model_badges.as_ref())
            .and_then(|b| b.songrow.as_ref().or(b.songcard.as_ref()))
            .and_then(|badge| badge.display_name.clone())
            .or_else(|| self.major_model_version.as_ref().map(|v| v.to_uppercase()))
            .unwrap_or_else(|| match self.model_name.as_deref() {
                Some("chirp-goose") => "V6".to_string(),
                Some("chirp-v4") => "V4".to_string(),
                Some("chirp-v3-5") => "V3.5".to_string(),
                Some("chirp-v3") => "V3".to_string(),
                Some(other) => other.to_string(),
                None => "Suno AI".to_string(),
            });

        let task_suffix = match self.metadata.as_ref().and_then(|m| m.task.as_deref()) {
            Some("cover") => " COVER",
            Some("upload_extend") | Some("extend") => " EXTEND",
            Some("remix") => " REMIX",
            _ => "",
        };

        format!("{}{}", base_model, task_suffix)
    }

    pub fn display_tags(&self) -> String {
        self.metadata
            .as_ref()
            .and_then(|m| m.tags.as_deref())
            .unwrap_or("N/A")
            .to_string()
    }

    pub fn display_prompt(&self) -> String {
        if let Some(m) = &self.metadata {
            if let Some(p) = &m.prompt {
                if !p.trim().is_empty() {
                    return p.clone();
                }
            }
            if let Some(gpt_p) = &m.gpt_description_prompt {
                if !gpt_p.trim().is_empty() {
                    return gpt_p.clone();
                }
            }
        }
        "[No prompt/lyrics]".to_string()
    }

    pub fn duration_formatted(&self) -> String {
        let seconds = self
            .metadata
            .as_ref()
            .and_then(|m| m.duration)
            .unwrap_or(0.0)
            .round() as u64;

        if seconds == 0 {
            "--:--".to_string()
        } else {
            let m = seconds / 60;
            let s = seconds % 60;
            format!("{:02}:{:02}", m, s)
        }
    }

    pub fn get_mango_m4a_url(&self) -> Option<String> {
        if let Some(media_list) = &self.media_urls {
            for m in media_list {
                if m.url.contains("cloudfront.net") || m.url.contains(".m4a") {
                    return Some(m.url.clone());
                }
            }
        }
        Some(format!(
            "https://d2lwuy8qc234o3.cloudfront.net/1/clip/{}.m4a",
            self.id
        ))
    }

    pub fn get_audio_url(&self) -> String {
        if let Some(url) = &self.audio_url {
            if !url.trim().is_empty() && !url.contains("/api/forbidden") {
                return url.clone();
            }
        }
        if let Some(video) = &self.video_url {
            if !video.trim().is_empty() {
                return video.clone();
            }
        }
        // Suno standard CDN fallback format (mp4 contains 192k AAC stereo audio)
        format!("https://cdn1.suno.ai/{}.mp4", self.id)
    }

    pub fn get_cover_url(&self) -> Option<String> {
        if let Some(url) = &self.image_large_url {
            if !url.trim().is_empty() {
                return Some(url.clone());
            }
        }
        if let Some(url) = &self.image_url {
            if !url.trim().is_empty() {
                return Some(url.clone());
            }
        }
        Some(format!("https://cdn1.suno.ai/image_{}.png", self.id))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Idle,
    Queued,
    Downloading {
        progress: f32,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    Completed {
        path: PathBuf,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct TrackItem {
    pub clip: Clip,
    pub selected: bool,
    pub download_status: DownloadStatus,
}

impl TrackItem {
    pub fn new(clip: Clip) -> Self {
        Self {
            clip,
            selected: false,
            download_status: DownloadStatus::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_model_cover_badge() {
        let clip: Clip = serde_json::from_str(
            r#"{
                "id": "123",
                "model_name": "chirp-goose",
                "metadata": {
                    "task": "cover",
                    "model_badges": {
                        "songrow": {
                            "display_name": "V6-MINI"
                        }
                    }
                }
            }"#,
        )
        .expect("deserialize clip");

        assert_eq!(clip.display_model(), "V6-MINI COVER");
    }

    #[test]
    fn test_display_model_fallback() {
        let clip: Clip = serde_json::from_str(
            r#"{
                "id": "456",
                "model_name": "chirp-v3-5"
            }"#,
        )
        .expect("deserialize clip");

        assert_eq!(clip.display_model(), "V3.5");
    }
}
