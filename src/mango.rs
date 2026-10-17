use aes::cipher::{KeyIvInit, StreamCipher};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::Aes256Gcm;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN, REFERER, USER_AGENT,
};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};

type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;
type Aes256Ctr = ctr::Ctr128BE<aes::Aes256>;

#[derive(Deserialize)]
struct MangoRightsResponse {
    key: String,
    iv: String,
}

pub struct MangoDecryptor {
    client: Client,
    token: String,
}

impl MangoDecryptor {
    pub fn new(token: String) -> Result<Self> {
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
            .context("Failed to build HTTP client for Mango")?;

        Ok(Self { client, token })
    }

    /// Fetch rights (encrypted AES-CTR key and IV) from Suno license endpoint
    pub async fn fetch_rights(&self, clip_id: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        let clean_jwt = crate::api::SunoClient::extract_token(&self.token);
        let url = "https://studio-api-prod.suno.com/api/mango/rights";
        let body = serde_json::json!({
            "content_params": {
                "content_id": clip_id,
                "content_type": "clip"
            }
        });

        let resp = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", clean_jwt))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to request mango rights")?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "Mango rights request failed with status {}",
                resp.status()
            ));
        }

        let rights: MangoRightsResponse = resp
            .json()
            .await
            .context("Failed to parse mango rights JSON")?;

        // Derive master key from token: SHA-256(token)
        let mut hasher = Sha256::new();
        hasher.update(clean_jwt.as_bytes());
        let master_key = hasher.finalize();

        let cipher_gcm = Aes256Gcm::new_from_slice(&master_key)
            .map_err(|e| anyhow!("Failed to init AES-GCM: {:?}", e))?;

        let enc_key = BASE64
            .decode(&rights.key)
            .context("Failed to decode base64 key")?;
        let enc_iv = BASE64
            .decode(&rights.iv)
            .context("Failed to decode base64 iv")?;

        if enc_key.len() < 12 || enc_iv.len() < 12 {
            return Err(anyhow!("Invalid mango encrypted key/iv length"));
        }

        // WebCrypto AES-GCM: first 12 bytes are IV, rest is ciphertext + tag
        let aad = clip_id.as_bytes();

        let nonce_key = aes_gcm::Nonce::try_from(&enc_key[..12])
            .map_err(|e| anyhow!("Invalid key nonce: {:?}", e))?;
        let payload_key = Payload {
            msg: &enc_key[12..],
            aad,
        };
        let audio_key = cipher_gcm
            .decrypt(&nonce_key, payload_key)
            .map_err(|e| anyhow!("Failed to decrypt audio key: {:?}", e))?;

        let nonce_iv = aes_gcm::Nonce::try_from(&enc_iv[..12])
            .map_err(|e| anyhow!("Invalid iv nonce: {:?}", e))?;
        let payload_iv = Payload {
            msg: &enc_iv[12..],
            aad,
        };
        let audio_iv = cipher_gcm
            .decrypt(&nonce_iv, payload_iv)
            .map_err(|e| anyhow!("Failed to decrypt audio iv: {:?}", e))?;

        Ok((audio_key, audio_iv))
    }

    /// Decrypt encrypted audio data using derived AES-CTR key and IV in place
    pub fn decrypt_stream(audio_key: &[u8], audio_iv: &[u8], ciphertext: &mut [u8]) -> Result<()> {
        if audio_iv.len() != 16 {
            return Err(anyhow!(
                "Invalid IV length for AES-CTR: expected 16, got {}",
                audio_iv.len()
            ));
        }

        match audio_key.len() {
            16 => {
                let mut cipher = Aes128Ctr::new_from_slices(audio_key, audio_iv)
                    .map_err(|e| anyhow!("Failed to init Aes128Ctr: {:?}", e))?;
                cipher.apply_keystream(ciphertext);
                Ok(())
            }
            32 => {
                let mut cipher = Aes256Ctr::new_from_slices(audio_key, audio_iv)
                    .map_err(|e| anyhow!("Failed to init Aes256Ctr: {:?}", e))?;
                cipher.apply_keystream(ciphertext);
                Ok(())
            }
            len => Err(anyhow!("Unsupported AES key length: {}", len)),
        }
    }
}
