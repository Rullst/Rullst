use super::BunnyStream;
use crate::{
    MediaError as Error, PlaybackGrant, PlaybackKind, ProviderMode, UploadGrant,
    VerifiedNotification, VideoId, WebhookHeaders, contracts::checked_time,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ring::{digest, hmac};
use serde::Deserialize;

pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        value.push(char::from(DIGITS[usize::from(b >> 4)]));
        value.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    value
}
pub(super) fn sha(parts: &[&str]) -> String {
    let mut hash = digest::Context::new(&digest::SHA256);
    for part in parts {
        hash.update(part.as_bytes());
    }
    hex(hash.finish().as_ref())
}
fn expiration(now: i64, ttl: u32, maximum: u32) -> Result<i64, Error> {
    checked_time(now)?;
    if ttl == 0 || ttl > maximum {
        return Err(Error::InvalidInput);
    }
    checked_time(now.checked_add(i64::from(ttl)).ok_or(Error::Clock)?)
}

impl BunnyStream {
    pub(super) fn upload_grant(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
    ) -> Result<UploadGrant, Error> {
        let expires_at = expiration(now, ttl, 3600)?;
        let mode = self.config.credentials.mode;
        let endpoint = match mode {
            ProviderMode::Offline => "https://rullst-media.invalid/tusupload".into(),
            _ => format!("{}/tusupload", self.origin),
        };
        let signature = sha(&[
            &self.config.library.value().to_string(),
            &self.config.credentials.api,
            &expires_at.to_string(),
            video.as_str(),
        ]);
        Ok(UploadGrant {
            endpoint,
            library: self.config.library,
            video: video.clone(),
            expires_at,
            signature,
            mode,
        })
    }

    pub(super) fn playback_grant(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, Error> {
        let expires_at = expiration(now, ttl, 900)?;
        let expires = expires_at.to_string();
        let mode = self.config.credentials.mode;
        let url = if mode == ProviderMode::Offline {
            format!(
                "https://rullst-media.invalid/videos/{}?expires={expires}",
                video.as_str()
            )
        } else {
            match kind {
                PlaybackKind::Embed => {
                    let token = sha(&[&self.config.credentials.embed, video.as_str(), &expires]);
                    let origin = if mode == ProviderMode::ProtocolFixture {
                        self.origin.as_str()
                    } else {
                        "https://iframe.mediadelivery.net"
                    };
                    format!(
                        "{origin}/embed/{}/{}?token={token}&expires={expires}",
                        self.config.library.value(),
                        video.as_str()
                    )
                }
                PlaybackKind::Hls => {
                    let path = format!("/{}/", video.as_str());
                    let params = format!("token_path={path}");
                    let key =
                        hmac::Key::new(hmac::HMAC_SHA256, self.config.credentials.cdn.as_bytes());
                    let message = format!("{path}{expires}{params}");
                    let token =
                        URL_SAFE_NO_PAD.encode(hmac::sign(&key, message.as_bytes()).as_ref());
                    let encoded_path = format!("%2F{}%2F", video.as_str());
                    let origin = if mode == ProviderMode::ProtocolFixture {
                        self.origin.clone()
                    } else {
                        format!("https://{}", self.config.cdn_host)
                    };
                    format!(
                        "{origin}/bcdn_token=HS256-{token}&expires={expires}&token_path={encoded_path}{path}playlist.m3u8"
                    )
                }
                PlaybackKind::Mp4_720p => {
                    let path = format!("/{}/play_720p.mp4", video.as_str());
                    let key =
                        hmac::Key::new(hmac::HMAC_SHA256, self.config.credentials.cdn.as_bytes());
                    let token = URL_SAFE_NO_PAD
                        .encode(hmac::sign(&key, format!("{path}{expires}").as_bytes()).as_ref());
                    let origin = if mode == ProviderMode::ProtocolFixture {
                        self.origin.clone()
                    } else {
                        format!("https://{}", self.config.cdn_host)
                    };
                    format!("{origin}{path}?token=HS256-{token}&expires={expires}")
                }
            }
        };
        Ok(PlaybackGrant {
            url,
            expires_at,
            mode,
        })
    }

    /// Verifies exact bytes and the distinct webhook protocol; does not mutate
    /// asset state or trust the notification status as current video readiness.
    pub fn verify_notification(
        &self,
        headers: WebhookHeaders<'_>,
        body: &[u8],
    ) -> Result<VerifiedNotification, Error> {
        if body.is_empty()
            || body.len() > 4096
            || headers.version != "v1"
            || headers.algorithm != "hmac-sha256"
            || headers.signature.len() != 64
            || !headers
                .signature
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::Signature);
        }
        let mut signature = [0u8; 32];
        for (i, pair) in headers
            .signature
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
        {
            fn nibble(b: u8) -> u8 {
                if b.is_ascii_digit() {
                    b - b'0'
                } else {
                    b - b'a' + 10
                }
            }
            signature[i] = (nibble(pair[0]) << 4) | nibble(pair[1]);
        }
        let key = hmac::Key::new(
            hmac::HMAC_SHA256,
            self.config.credentials.webhook.as_bytes(),
        );
        hmac::verify(&key, body, &signature).map_err(|_| Error::Signature)?;
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase", deny_unknown_fields)]
        struct Payload {
            video_library_id: i64,
            video_guid: VideoId,
            status: u8,
        }
        let payload: Payload = serde_json::from_slice(body).map_err(|_| Error::Protocol)?;
        if payload.video_library_id != self.config.library.value() || payload.status > 10 {
            return Err(Error::Protocol);
        }
        Ok(VerifiedNotification {
            library: self.config.library,
            video: payload.video_guid,
            digest: hex(digest::digest(&digest::SHA256, body).as_ref()),
            mode: self.config.credentials.mode,
        })
    }
}
