use crate::{
    LibraryId, MediaError as Error, Processing, RemoteVideo, VideoId, provider::validate_remote,
};
use serde::{Deserialize, Serialize};

/// Bunny's UpdateVideoModel has no description property. API 1.6.6 documents
/// that a meta tag with this property also updates the separate Description
/// field, which Smart Generate may rewrite independently. The adapter writes
/// and verifies only this tag, never the top-level field.
const DESCRIPTION: &str = "description";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Video {
    video_library_id: LibraryId,
    guid: VideoId,
    title: String,
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
        if let Some(tag) = tags.iter_mut().find(|tag| tag.property == DESCRIPTION) {
            tag.value = Some(description.into());
        } else {
            if tags.len() == 50 {
                return Err(Error::Capacity);
            }
            tags.push(MetaTag {
                property: DESCRIPTION.into(),
                value: Some(description.into()),
            });
        }
        Ok(tags)
    }
    pub fn checked(self, library: LibraryId) -> Result<RemoteVideo, Error> {
        let tags = self.meta_tags.unwrap_or_default();
        validate_tags(&tags)?;
        let description = tags
            .into_iter()
            .find(|tag| tag.property == DESCRIPTION)
            .and_then(|tag| tag.value)
            .unwrap_or_default();
        let processing = processing(self.status)?;
        let resolutions = self.available_resolutions.unwrap_or_default();
        if resolutions.len() > 128 {
            return Err(Error::Protocol);
        }
        let video = RemoteVideo {
            id: self.guid,
            library: self.video_library_id,
            title: self.title,
            description,
            processing,
            length_seconds: self.length,
            mp4_720p: self.has_mp4_fallback && resolutions.split(',').any(|r| r == "720p"),
        };
        validate_remote(&video, library)?;
        Ok(video)
    }
}

/// The API status codes (`VideoModelStatus`); only Finished (4) is `Ready`.
/// Webhook codes are a different numbering and never reach this function.
///
/// 7 (`JitSegmenting`) is still `Processing`: Bunny's just-in-time encoding
/// creates the first segments before any playback can start. 8
/// (`JitPlaylistsCreated`) is `Playable`: the playlists exist, so Bunny's
/// player streams the video while encoding goes on.
pub(super) fn processing(status: u8) -> Result<Processing, Error> {
    Ok(match status {
        0 => Processing::AwaitingUpload,
        1..=3 | 7 => Processing::Processing,
        8 => Processing::Playable,
        4 => Processing::Ready,
        5..=6 => Processing::Failed,
        _ => return Err(Error::Protocol),
    })
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    fn checked(changes: serde_json::Value) -> Result<RemoteVideo, Error> {
        let mut video = json!({
            "videoLibraryId": 7,
            "guid": "12345678-1234-4234-8234-000000000001",
            "title": "marker",
            "status": 4,
            "length": 12,
            "hasMP4Fallback": true,
            "availableResolutions": "720p",
        });
        for (key, value) in changes.as_object().unwrap() {
            video[key] = value.clone();
        }
        serde_json::from_value::<Video>(video)
            .unwrap()
            .checked(LibraryId::new(7).unwrap())
    }

    fn tags(tags: serde_json::Value) -> Result<RemoteVideo, Error> {
        checked(json!({ "metaTags": tags }))
    }

    #[test]
    fn resolution_lists_are_bounded_at_128_bytes() {
        let longest = format!("720p,{}", "x".repeat(123));
        assert_eq!(longest.len(), 128);
        assert!(
            checked(json!({ "availableResolutions": longest }))
                .unwrap()
                .mp4_720p
        );
        assert_eq!(
            checked(json!({ "availableResolutions": format!("{longest}x") })),
            Err(Error::Protocol)
        );
    }

    #[test]
    fn meta_tags_are_validated_one_rule_at_a_time() {
        let video = tags(json!([
            { "property": "p".repeat(100), "value": "v".repeat(4096) },
            { "property": "description", "value": "about" },
            { "property": "a.b_c:d'e\"f-1", "value": null },
        ]))
        .unwrap();
        assert_eq!(video.description, "about");
        for invalid in [
            json!([{ "property": "", "value": "v" }]),
            json!([{ "property": "p".repeat(101), "value": "v" }]),
            json!([{ "property": "a/b", "value": "v" }]),
            json!([{ "property": "p", "value": "v".repeat(4097) }]),
            json!([{ "property": "p", "value": "1" }, { "property": "p", "value": "2" }]),
        ] {
            assert_eq!(tags(invalid.clone()), Err(Error::Protocol), "{invalid}");
        }
    }
}
