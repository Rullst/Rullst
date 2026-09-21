use crate::{
    LibraryId, MediaError as Error, Processing, RemoteVideo, VideoId, provider::validate_remote,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Video {
    video_library_id: LibraryId,
    guid: VideoId,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: u8,
    length: u32,
    #[serde(rename = "hasMP4Fallback")]
    has_mp4_fallback: bool,
    #[serde(default)]
    available_resolutions: Option<String>,
    #[serde(default)]
    meta_tags: Option<Vec<MetaTag>>,
}
impl Video {
    pub fn update_tags(&mut self, description: &str) -> Result<Vec<MetaTag>, Error> {
        let mut tags = self.meta_tags.take().unwrap_or_default();
        validate_tags(&tags)?;
        if let Some(tag) = tags.iter_mut().find(|tag| tag.property == "description") {
            tag.value = Some(description.into());
        } else {
            if tags.len() == 50 {
                return Err(Error::Capacity);
            }
            tags.push(MetaTag {
                property: "description".into(),
                value: Some(description.into()),
            });
        }
        Ok(tags)
    }
    pub fn checked(self, library: LibraryId) -> Result<RemoteVideo, Error> {
        validate_tags(self.meta_tags.as_deref().unwrap_or_default())?;
        let processing = match self.status {
            0 => Processing::AwaitingUpload,
            1..=3 | 7..=8 => Processing::Processing,
            4 => Processing::Ready,
            5..=6 => Processing::Failed,
            _ => return Err(Error::Protocol),
        };
        let resolutions = self.available_resolutions.unwrap_or_default();
        if resolutions.len() > 128 {
            return Err(Error::Protocol);
        }
        let video = RemoteVideo {
            id: self.guid,
            library: self.video_library_id,
            title: self.title,
            description: self.description.unwrap_or_default(),
            processing,
            length_seconds: self.length,
            mp4_720p: self.has_mp4_fallback && resolutions.split(',').any(|r| r == "720p"),
        };
        validate_remote(&video, library)?;
        Ok(video)
    }
}

#[derive(Deserialize, Serialize)]
pub(super) struct MetaTag {
    property: String,
    value: Option<String>,
}
fn validate_tags(tags: &[MetaTag]) -> Result<(), Error> {
    let mut names = std::collections::BTreeSet::new();
    if tags.len() > 50 {
        return Err(Error::Protocol);
    }
    for tag in tags {
        if tag.property.is_empty()
            || tag.property.len() > 100
            || !tag
                .property
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:'\"-".contains(&b))
            || tag.value.as_ref().is_some_and(|value| value.len() > 4096)
            || !names.insert(&tag.property)
        {
            return Err(Error::Protocol);
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Page {
    pub total_items: u32,
    pub current_page: u32,
    pub items_per_page: u32,
    pub items: Vec<Video>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Status {
    pub success: bool,
    pub status_code: u16,
}
