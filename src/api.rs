use crate::models::Clip;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, COOKIE, ORIGIN, REFERER, USER_AGENT,
};
use reqwest::Client;
use serde_json::Value;

const BASE_API_URL: &str = "https://studio-api.prod.suno.com";
const FALLBACK_API_URL: &str = "https://studio-api.suno.ai";

pub struct SunoClient {
    client: Client,
    token: Option<String>,
}

impl SunoClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(ORIGIN, HeaderValue::from_static("https://suno.com"));
        headers.insert(REFERER, HeaderValue::from_static("https://suno.com/"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client, token })
    }

    #[allow(dead_code)]
    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    pub fn has_token(&self) -> bool {
        self.token
            .as_ref()
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn extract_token(input: &str) -> String {
        let trimmed = input.trim();
        // If user pasted a full cookie string containing __session=...
        if let Some(pos) = trimmed.find("__session=") {
            let after = &trimmed[pos + "__session=".len()..];
            let end = after.find(';').unwrap_or(after.len());
            return after[..end].trim().to_string();
        }
        if let Some(stripped) = trimmed.strip_prefix("Bearer ") {
            return stripped.trim().to_string();
        }
        trimmed.to_string()
    }

    fn apply_auth(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(raw_token) = &self.token {
            let trimmed = raw_token.trim();
            if !trimmed.is_empty() {
                let clean_jwt = Self::extract_token(trimmed);
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", clean_jwt)) {
                    req = req.header(AUTHORIZATION, val);
                }

                // Also pass cookie header if user provided a cookie
                if trimmed.contains('=') {
                    if let Ok(cookie_val) = HeaderValue::from_str(trimmed) {
                        req = req.header(COOKIE, cookie_val);
                    }
                }
            }
        }
        req
    }

    /// Extract UUID from URL, share link, or raw string
    pub fn extract_uuid(input: &str) -> Option<String> {
        let trimmed = input.trim();
        let re = Regex::new(
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        )
        .ok()?;
        re.find(trimmed).map(|m| m.as_str().to_lowercase())
    }

    /// Fetch a single clip by its UUID (supports public clips even without auth token)
    pub async fn fetch_clip(&self, uuid: &str) -> Result<Clip> {
        let clean_uuid = Self::extract_uuid(uuid).unwrap_or_else(|| uuid.to_string());

        // Attempt 1: /api/clip/{id}
        let url = format!("{}/api/clip/{}", BASE_API_URL, clean_uuid);
        let req = self.apply_auth(self.client.get(&url));
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(clip) = resp.json::<Clip>().await {
                    return Ok(clip);
                }
            }
        }

        // Attempt 2: /api/feed/?ids={id}
        let url_feed = format!("{}/api/feed/?ids={}", BASE_API_URL, clean_uuid);
        let req2 = self.apply_auth(self.client.get(&url_feed));
        if let Ok(resp) = req2.send().await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                // Can be an array of clips, or an object with clips
                if let Ok(clips) = serde_json::from_str::<Vec<Clip>>(&text) {
                    if let Some(clip) = clips.into_iter().next() {
                        return Ok(clip);
                    }
                }
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    if let Some(arr) = val.get("clips").and_then(|c| c.as_array()) {
                        if let Some(first) = arr.first() {
                            if let Ok(clip) = serde_json::from_value::<Clip>(first.clone()) {
                                return Ok(clip);
                            }
                        }
                    }
                }
            }
        }

        // Attempt 3: Fallback domain
        let url_fallback = format!("{}/api/clip/{}", FALLBACK_API_URL, clean_uuid);
        let req3 = self.apply_auth(self.client.get(&url_fallback));
        let resp3 = req3
            .send()
            .await
            .context("Failed to request clip from Suno API")?;
        if !resp3.status().is_success() {
            let status = resp3.status();
            return Err(anyhow!(
                "Failed to fetch clip {}: HTTP {}",
                clean_uuid,
                status
            ));
        }

        let clip = resp3
            .json::<Clip>()
            .await
            .context("Failed to parse clip JSON")?;
        Ok(clip)
    }

    /// Fetch personal library / feed page (requires authentication)
    pub async fn fetch_feed_page(&self, page: usize) -> Result<Vec<Clip>> {
        if !self.has_token() {
            return Err(anyhow!(
                "No session token configured. Please set your Suno session token in settings [c] to load your personal library."
            ));
        }

        // Attempt 1: Modern Suno API /api/feed/v3 (POST with JSON filters)
        let url_v3 = format!("{}/api/feed/v3", BASE_API_URL);
        let body_v3 = serde_json::json!({
            "cursor": null,
            "limit": 50,
            "filters": {
                "disliked": "False",
                "trashed": "False",
                "fromStudioProject": { "presence": "False" },
                "stem": { "presence": "False" },
                "stemComplement": "False"
            }
        });

        let req_v3 = self.apply_auth(self.client.post(&url_v3).json(&body_v3));
        if let Ok(resp) = req_v3.send().await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    if let Some(arr) = val.get("clips").and_then(|c| c.as_array()) {
                        let clips: Vec<Clip> = arr
                            .iter()
                            .filter_map(|c| serde_json::from_value::<Clip>(c.clone()).ok())
                            .collect();
                        if !clips.is_empty() {
                            return Ok(clips);
                        }
                    }
                }
            }
        }

        // Attempt 2: Legacy /api/feed/v2?page={page}
        let url_v2 = format!("{}/api/feed/v2?page={}", BASE_API_URL, page);
        let req_v2 = self.apply_auth(self.client.get(&url_v2));
        if let Ok(resp) = req_v2.send().await {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                if let Ok(clips) = serde_json::from_str::<Vec<Clip>>(&text) {
                    return Ok(clips);
                }
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    if let Some(arr) = val.get("clips").and_then(|c| c.as_array()) {
                        let clips: Vec<Clip> = arr
                            .iter()
                            .filter_map(|c| serde_json::from_value::<Clip>(c.clone()).ok())
                            .collect();
                        if !clips.is_empty() {
                            return Ok(clips);
                        }
                    }
                }
            }
        }

        // Try /api/feed/?page={page}
        let url_v1 = format!("{}/api/feed/?page={}", BASE_API_URL, page);
        let req_v1 = self.apply_auth(self.client.get(&url_v1));
        let resp = req_v1
            .send()
            .await
            .context("Failed to connect to Suno feed endpoint")?;

        if !resp.status().is_success() {
            let status = resp.status();
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                return Err(anyhow!(
                    "Authentication failed (HTTP {}). Your token might be expired. Please update it in Settings [c].",
                    status
                ));
            }
            return Err(anyhow!("Feed request failed with HTTP {}", status));
        }

        let text = resp.text().await.context("Failed to read feed response")?;
        if let Ok(clips) = serde_json::from_str::<Vec<Clip>>(&text) {
            return Ok(clips);
        }

        if let Ok(val) = serde_json::from_str::<Value>(&text) {
            if let Some(arr) = val.get("clips").and_then(|c| c.as_array()) {
                let clips: Vec<Clip> = arr
                    .iter()
                    .filter_map(|c| serde_json::from_value::<Clip>(c.clone()).ok())
                    .collect();
                return Ok(clips);
            }
        }

        Err(anyhow!("Could not parse clips from Suno feed response"))
    }
}
