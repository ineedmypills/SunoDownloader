use crate::config::Config;
use crate::models::Clip;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use id3::frame::{Comment, Lyrics, Picture, PictureType};
use id3::{Tag, TagLike, Version};
use reqwest::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum DownloadProgressMsg {
    Started {
        clip_id: String,
    },
    Progress {
        clip_id: String,
        progress: f32,
        downloaded: u64,
        total: u64,
    },
    Completed {
        clip_id: String,
        file_path: PathBuf,
    },
    Failed {
        clip_id: String,
        error: String,
    },
}

pub struct Downloader {
    client: Client,
    config: Config,
}

impl Downloader {
    pub fn new(config: Config) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client, config }
    }

    #[allow(dead_code)]
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Prepare destination audio file path
    pub fn get_target_path(&self, clip: &Clip) -> PathBuf {
        let title = clip.display_title();
        let artist = clip.display_artist();
        let style = clip.display_tags();
        let filename = self
            .config
            .format_filename(&title, &clip.id, &artist, &style);
        self.config.download_dir.join(format!("{}.mp3", filename))
    }

    /// Download a single clip with progress reporting
    pub async fn download_clip(
        &self,
        clip: &Clip,
        sender: Option<UnboundedSender<DownloadProgressMsg>>,
    ) -> Result<PathBuf> {
        let clip_id = clip.id.clone();
        if let Some(s) = &sender {
            let _ = s.send(DownloadProgressMsg::Started {
                clip_id: clip_id.clone(),
            });
        }

        // Ensure target directory exists
        fs::create_dir_all(&self.config.download_dir).with_context(|| {
            format!(
                "Failed to create download dir: {:?}",
                self.config.download_dir
            )
        })?;

        let target_path = self.get_target_path(clip);

        // 1. Attempt Suno Mango DRM Decryption if session token is configured
        if let Some(token) = &self.config.session_token {
            if let Ok(decryptor) = crate::mango::MangoDecryptor::new(token.clone()) {
                if let Ok((audio_key, audio_iv)) = decryptor.fetch_rights(&clip.id).await {
                    let mango_url = clip.get_mango_m4a_url().unwrap_or_else(|| {
                        format!(
                            "https://d2lwuy8qc234o3.cloudfront.net/1/clip/{}.m4a",
                            clip.id
                        )
                    });

                    let temp_dec_path = self
                        .config
                        .download_dir
                        .join(format!("{}.dec.m4a", clip.id));

                    if let Ok(resp) = self.client.get(&mango_url).send().await {
                        if resp.status().is_success() {
                            let total_size = resp.content_length().unwrap_or(0);
                            let mut downloaded: u64 = 0;
                            let mut stream = resp.bytes_stream();
                            let mut enc_bytes = Vec::new();

                            while let Some(chunk_res) = stream.next().await {
                                if let Ok(chunk) = chunk_res {
                                    enc_bytes.extend_from_slice(&chunk);
                                    downloaded += chunk.len() as u64;

                                    if total_size > 0 {
                                        let progress = downloaded as f32 / total_size as f32;
                                        if let Some(s) = &sender {
                                            let _ = s.send(DownloadProgressMsg::Progress {
                                                clip_id: clip_id.clone(),
                                                progress,
                                                downloaded,
                                                total: total_size,
                                            });
                                        }
                                    }
                                }
                            }

                            // Decrypt in place
                            if crate::mango::MangoDecryptor::decrypt_stream(
                                &audio_key,
                                &audio_iv,
                                &mut enc_bytes,
                            )
                            .is_ok()
                            {
                                let _ = fs::write(&temp_dec_path, &enc_bytes);

                                // Convert decrypted m4a to pristine 320k mp3 using ffmpeg
                                let mut cmd = std::process::Command::new("ffmpeg");
                                cmd.arg("-y")
                                    .arg("-i")
                                    .arg(&temp_dec_path)
                                    .arg("-vn")
                                    .arg("-c:a")
                                    .arg("libmp3lame")
                                    .arg("-b:a")
                                    .arg("320k")
                                    .arg("-id3v2_version")
                                    .arg("3")
                                    .arg("-metadata")
                                    .arg(format!("title={}", clip.display_title()))
                                    .arg("-metadata")
                                    .arg(format!("artist={}", clip.display_artist()))
                                    .arg("-metadata")
                                    .arg("album=Suno Creations")
                                    .arg("-metadata")
                                    .arg(format!("genre={}", clip.display_tags()))
                                    .arg("-metadata")
                                    .arg(format!(
                                        "comment=Model: {} | Clip ID: {}",
                                        clip.display_model(),
                                        clip.id
                                    ))
                                    .arg(&target_path);
                                let ffmpeg_res = cmd.output();

                                let _ = fs::remove_file(&temp_dec_path);

                                let final_path = match ffmpeg_res {
                                    Ok(out) if out.status.success() => target_path,
                                    _ => {
                                        let alt_path = target_path.with_extension("m4a");
                                        let _ = fs::write(&alt_path, &enc_bytes);
                                        alt_path
                                    }
                                };

                                return self.finalize_download(clip, final_path, sender).await;
                            }
                        }
                    }
                }
            }
        }

        // 2. Standard direct stream fallback (for public clips, mp4 video or direct audio)
        let audio_url = clip.get_audio_url();
        let is_direct_mp3 = audio_url.ends_with(".mp3");

        let temp_filename = if is_direct_mp3 {
            format!("{}.part.mp3", clip.id)
        } else {
            format!("{}.part.mp4", clip.id)
        };
        let temp_path = self.config.download_dir.join(&temp_filename);

        let response = self
            .client
            .get(&audio_url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to audio URL: {}", audio_url))?;

        if !response.status().is_success() {
            let err = format!(
                "HTTP error {} fetching audio from {}",
                response.status(),
                audio_url
            );
            if let Some(s) = &sender {
                let _ = s.send(DownloadProgressMsg::Failed {
                    clip_id: clip_id.clone(),
                    error: err.clone(),
                });
            }
            return Err(anyhow!(err));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        let mut file = File::create(&temp_path)
            .with_context(|| format!("Failed to create temporary file: {:?}", temp_path))?;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.with_context(|| "Error while reading stream chunks")?;
            file.write_all(&chunk)
                .with_context(|| "Failed to write bytes to temp file")?;

            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let progress = downloaded as f32 / total_size as f32;
                if let Some(s) = &sender {
                    let _ = s.send(DownloadProgressMsg::Progress {
                        clip_id: clip_id.clone(),
                        progress,
                        downloaded,
                        total: total_size,
                    });
                }
            }
        }

        file.flush().context("Failed to flush audio file")?;
        drop(file);

        // Convert or finalize audio file to MP3
        let final_path = if is_direct_mp3 {
            fs::rename(&temp_path, &target_path)
                .with_context(|| format!("Failed to finalize audio file to {:?}", target_path))?;
            target_path
        } else {
            let mut cmd = std::process::Command::new("ffmpeg");
            cmd.arg("-y")
                .arg("-i")
                .arg(&temp_path)
                .arg("-vn")
                .arg("-c:a")
                .arg("libmp3lame")
                .arg("-b:a")
                .arg("320k")
                .arg("-id3v2_version")
                .arg("3")
                .arg("-metadata")
                .arg(format!("title={}", clip.display_title()))
                .arg("-metadata")
                .arg(format!("artist={}", clip.display_artist()))
                .arg("-metadata")
                .arg("album=Suno Creations")
                .arg("-metadata")
                .arg(format!("genre={}", clip.display_tags()))
                .arg("-metadata")
                .arg(format!(
                    "comment=Model: {} | Clip ID: {}",
                    clip.display_model(),
                    clip.id
                ))
                .arg(&target_path);
            let ffmpeg_res = cmd.output();

            match ffmpeg_res {
                Ok(out) if out.status.success() => {
                    let _ = fs::remove_file(&temp_path);
                    target_path
                }
                _ => {
                    let alt_path = target_path.with_extension("mp4");
                    let _ = fs::rename(&temp_path, &alt_path);
                    alt_path
                }
            }
        };

        self.finalize_download(clip, final_path, sender).await
    }

    async fn finalize_download(
        &self,
        clip: &Clip,
        final_path: PathBuf,
        sender: Option<UnboundedSender<DownloadProgressMsg>>,
    ) -> Result<PathBuf> {
        let clip_id = clip.id.clone();

        // Download cover image if needed for ID3 or separate file
        let cover_data = if self.config.embed_id3 || self.config.save_cover_image {
            self.fetch_cover_bytes(clip).await
        } else {
            None
        };

        // Save separate cover image if configured
        if self.config.save_cover_image {
            if let Some((bytes, ext)) = &cover_data {
                let cover_path = final_path.with_extension(ext);
                let _ = fs::write(cover_path, bytes);
            }
        }

        // Save separate lyrics file if configured
        if self.config.save_lyrics_file {
            let lyrics_content = format!(
                "Title: {}\nArtist: {}\nStyle: {}\nModel: {}\n\n--- Lyrics / Prompt ---\n{}",
                clip.display_title(),
                clip.display_artist(),
                clip.display_tags(),
                clip.display_model(),
                clip.display_prompt()
            );
            let lyrics_path = final_path.with_extension("txt");
            let _ = fs::write(lyrics_path, lyrics_content);
        }

        // Embed ID3 tags (only on mp3)
        if self.config.embed_id3 && final_path.extension().and_then(|e| e.to_str()) == Some("mp3") {
            if let Err(err) = self.embed_id3_tags(&final_path, clip, cover_data.as_ref()) {
                eprintln!(
                    "Warning: Failed to embed ID3 tags into {}: {:#}",
                    final_path.display(),
                    err
                );
            }
        }

        if let Some(s) = &sender {
            let _ = s.send(DownloadProgressMsg::Completed {
                clip_id: clip_id.clone(),
                file_path: final_path.clone(),
            });
        }

        Ok(final_path)
    }

    async fn fetch_cover_bytes(&self, clip: &Clip) -> Option<(Vec<u8>, String)> {
        let url = clip.get_cover_url()?;
        let mut req = self.client.get(&url);
        if let Some(tok) = &self.config.session_token {
            let clean_jwt = crate::api::SunoClient::extract_token(tok);
            if !clean_jwt.is_empty() {
                req = req.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", clean_jwt),
                );
            }
        }

        let resp = req.send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let ext = if content_type.contains("png") {
            "png".to_string()
        } else {
            "jpg".to_string()
        };

        let bytes = resp.bytes().await.ok()?.to_vec();
        Some((bytes, ext))
    }

    fn embed_id3_tags(
        &self,
        path: &Path,
        clip: &Clip,
        cover_data: Option<&(Vec<u8>, String)>,
    ) -> Result<()> {
        let mut tag = Tag::read_from_path(path).unwrap_or_else(|_| Tag::new());

        tag.set_title(clip.display_title());
        tag.set_artist(clip.display_artist());
        tag.set_album("Suno Creations");

        let tags = clip.display_tags();
        if !tags.is_empty() && tags != "N/A" {
            tag.set_genre(tags);
        }

        let prompt = clip.display_prompt();
        if !prompt.is_empty() && prompt != "[No prompt/lyrics]" {
            tag.add_frame(Lyrics {
                lang: "eng".to_string(),
                description: "Lyrics".to_string(),
                text: prompt.clone(),
            });
        }

        let model = clip.display_model();
        tag.add_frame(Comment {
            lang: "eng".to_string(),
            description: "Suno Details".to_string(),
            text: format!("Model: {}\nClip ID: {}", model, clip.id),
        });

        if let Some((bytes, ext)) = cover_data {
            let mime = if ext == "png" {
                "image/png"
            } else {
                "image/jpeg"
            };

            tag.add_frame(Picture {
                mime_type: mime.to_string(),
                picture_type: PictureType::CoverFront,
                description: "Front Cover".to_string(),
                data: bytes.clone(),
            });
        }

        // Id3v23 provides 100% universal compatibility across all audio players, Windows Explorer, macOS, and Linux
        tag.write_to_path(path, Version::Id3v23)
            .context("Failed to write ID3v2.3 tags")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_id3() {
        let dl = Downloader::new(Config::default());
        let clip = Clip {
            id: "test-clip-123".to_string(),
            title: Some("Test Song".to_string()),
            display_name: None,
            handle: None,
            created_at: None,
            status: Some("complete".to_string()),
            audio_url: None,
            video_url: None,
            media_urls: None,
            image_url: None,
            image_large_url: None,
            major_model_version: Some("v4".to_string()),
            model_name: Some("chirp-v4".to_string()),
            metadata: None,
            is_liked: None,
            is_trashed: None,
        };

        let path = PathBuf::from("/tmp/test.mp3");
        if path.exists() {
            let fake_cover = (
                vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46],
                "jpg".to_string(),
            );
            let res = dl.embed_id3_tags(&path, &clip, Some(&fake_cover));
            assert!(res.is_ok(), "ID3 embedding failed: {:?}", res.err());

            let read_tag = Tag::read_from_path(&path).expect("Failed to read tag back");
            assert_eq!(read_tag.title(), Some("Test Song"));
            assert_eq!(read_tag.artist(), Some("Suno AI"));
            assert_eq!(read_tag.album(), Some("Suno Creations"));
            assert_eq!(read_tag.pictures().count(), 1);
        }
    }
}
