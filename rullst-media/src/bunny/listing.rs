//! `GET /library/{id}/videos`: one validated page of the library (v13).

use super::{BunnyStream, wire::processing};
use crate::{
    LibraryId, ListOrder, ListQuery, ListedVideo, MediaError as Error, Processing, ProviderMode,
    VideoId, VideoPage, contracts::checked_time,
};
use reqwest::Url;
use serde::Deserialize;

const MAX_TITLE_BYTES: usize = 1024;
const MAX_URL_BYTES: usize = 2048;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    total_items: u64,
    current_page: u64,
    items_per_page: u64,
    #[serde(default)]
    items: Option<Vec<Item>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    video_library_id: LibraryId,
    guid: VideoId,
    title: String,
    status: u8,
    length: u32,
    #[serde(default)]
    date_uploaded: Option<String>,
    #[serde(default)]
    thumbnail_url: Option<String>,
}

impl Item {
    fn checked(self, library: LibraryId) -> Result<ListedVideo, Error> {
        if self.video_library_id != library
            || self.title.is_empty()
            || self.title.len() > MAX_TITLE_BYTES
            || self.length > 604_800
        {
            return Err(Error::Protocol);
        }
        Ok(ListedVideo {
            id: self.guid,
            title: self.title,
            length_seconds: self.length,
            processing: processing(self.status)?,
            created_at: match self.date_uploaded.as_deref() {
                None | Some("") => None,
                Some(text) => Some(utc_seconds(text).ok_or(Error::Protocol)?),
            },
            thumbnail_url: match self.thumbnail_url.as_deref() {
                None | Some("") => None,
                Some(text) => Some(thumbnail(text).ok_or(Error::Protocol)?),
            },
        })
    }
}

/// An absolute `https` URL without credentials or fragment.
fn thumbnail(text: &str) -> Option<String> {
    let url = Url::parse(text).ok()?;
    (text.len() <= MAX_URL_BYTES
        && url.scheme() == "https"
        && url.host_str().is_some_and(|host| !host.is_empty())
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none())
    .then(|| url.into())
}

/// `YYYY-MM-DDTHH:MM:SS`, optional `.fraction` (dropped) and an optional `Z`
/// or `±HH:MM` offset; without an offset the time is UTC, as Bunny sends it.
pub(super) fn utc_seconds(text: &str) -> Option<i64> {
    let (date, time) = text.split_once('T')?;
    let field = |part: &str, len: usize| -> Option<i64> {
        (part.len() == len && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse().ok())
            .flatten()
    };
    let mut ymd = date.split('-');
    let (year, month, day) = (
        field(ymd.next()?, 4)?,
        field(ymd.next()?, 2)?,
        field(ymd.next()?, 2)?,
    );
    if ymd.next().is_some() {
        return None;
    }
    let (clock, offset) = match time.find(['Z', '+', '-']) {
        Some(index) => time.split_at(index),
        None => (time, ""),
    };
    let offset = match offset {
        "" | "Z" => 0,
        _ => {
            let sign = if offset.starts_with('-') { -1 } else { 1 };
            let (hours, minutes) = offset.get(1..)?.split_once(':')?;
            let (hours, minutes) = (field(hours, 2)?, field(minutes, 2)?);
            if hours > 23 || minutes > 59 {
                return None;
            }
            sign * (hours * 3600 + minutes * 60)
        }
    };
    let (clock, fraction) = clock.split_once('.').unwrap_or((clock, "0"));
    if fraction.is_empty() || fraction.len() > 9 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut hms = clock.split(':');
    let (hour, minute, second) = (
        field(hms.next()?, 2)?,
        field(hms.next()?, 2)?,
        field(hms.next()?, 2)?,
    );
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return None,
    };
    if hms.next().is_some()
        || day < 1
        || day > days_in_month
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Days from 1970-01-01 to the civil date (proleptic Gregorian).
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    checked_time(days * 86_400 + hour * 3600 + minute * 60 + second - offset).ok()
}

/// The deterministic offline library: three sample videos, never stored.
const SAMPLES: [(&str, &str, u32, Processing, i64); 3] = [
    (
        "0c1f8a52-3d4e-4b6a-9f7e-2a1b3c4d5e6f",
        "Getting started",
        754,
        Processing::Ready,
        1_793_534_400,
    ),
    (
        "7d2e9b13-5a6c-4e8f-8b1d-3c4e5f6a7b8c",
        "Ownership in Rust",
        1_830,
        Processing::Processing,
        1_793_213_100,
    ),
    (
        "9e3fac24-6b7d-4f90-9c2e-4d5f6a7b8c9d",
        "Failed upload",
        0,
        Processing::Failed,
        1_792_488_600,
    ),
];

fn offline(query: &ListQuery) -> Result<VideoPage, Error> {
    let search = query.search().map(str::to_lowercase);
    let mut matches = Vec::new();
    for (id, title, length, processing, created) in SAMPLES {
        if search
            .as_ref()
            .is_none_or(|term| title.to_lowercase().contains(term))
        {
            matches.push(ListedVideo {
                id: VideoId::new(id)?,
                title: title.to_owned(),
                length_seconds: length,
                processing,
                created_at: Some(created),
                thumbnail_url: Some(format!("https://rullst-media.invalid/{id}/thumbnail.jpg")),
            });
        }
    }
    match query.order() {
        ListOrder::Title => matches.sort_by(|a, b| a.title.cmp(&b.title)),
        _ => matches.sort_by_key(|video| std::cmp::Reverse(video.created_at)),
    }
    let total = matches.len() as u64;
    let skip = usize::try_from((query.page() - 1).saturating_mul(query.per_page()))
        .map_err(|_| Error::InvalidInput)?;
    let items = matches
        .into_iter()
        .skip(skip)
        .take(query.per_page() as usize)
        .collect();
    Ok(VideoPage {
        items,
        page: query.page(),
        per_page: query.per_page(),
        total,
    })
}

impl BunnyStream {
    pub(super) async fn list_page(&self, query: &ListQuery) -> Result<VideoPage, Error> {
        if self.config.credentials.mode == ProviderMode::Offline {
            return offline(query);
        }
        let mut url = Url::parse("https://rullst-media.invalid/").map_err(|_| Error::Protocol)?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs
                .append_pair("page", &query.page().to_string())
                .append_pair("itemsPerPage", &query.per_page().to_string())
                .append_pair(
                    "orderBy",
                    match query.order() {
                        ListOrder::Title => "title",
                        _ => "date",
                    },
                );
            if let Some(term) = query.search() {
                pairs.append_pair("search", term);
            }
        }
        let path = format!("{}?{}", self.path(None), url.query().unwrap_or_default());
        let page: Page = self.read(&path).await?;
        let items = page.items.unwrap_or_default();
        if page.current_page != u64::from(query.page())
            || page.items_per_page != u64::from(query.per_page())
            || items.len() > query.per_page() as usize
            || page.total_items < items.len() as u64
        {
            return Err(Error::Protocol);
        }
        let library = self.config.library;
        Ok(VideoPage {
            items: items
                .into_iter()
                .map(|item| item.checked(library))
                .collect::<Result<_, _>>()?,
            page: query.page(),
            per_page: query.per_page(),
            total: page.total_items,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn upload_dates_parse_as_utc_seconds() {
        for (text, expected) in [
            ("2026-11-01T12:00:00", 1_793_534_400),
            ("2026-11-01T12:00:00.123", 1_793_534_400),
            ("2026-11-01T12:00:00.1234567Z", 1_793_534_400),
            ("2026-11-01T12:00:00.123456789Z", 1_793_534_400),
            ("2026-11-02T11:59:00+23:59", 1_793_534_400),
            ("2000-02-29T12:00:00", 951_825_600),
            ("2026-11-01T14:30:00+02:30", 1_793_534_400),
            ("2026-11-01T09:00:00-03:00", 1_793_534_400),
            ("2024-02-29T00:00:00Z", 1_709_164_800),
            ("1970-01-01T00:00:01", 1),
            ("2000-03-01T00:00:00", 951_868_800),
            ("2100-12-31T23:59:59", 4_133_980_799),
        ] {
            assert_eq!(utc_seconds(text), Some(expected), "{text}");
        }
        for text in [
            "",
            "2026-11-01",
            "2026-11-01 12:00:00",
            "2026-13-01T00:00:00",
            "2026-02-29T00:00:00",
            "2100-02-29T00:00:00",
            "2026-04-31T00:00:00",
            "2026-11-00T00:00:00",
            "2026-11-01T24:00:00",
            "2026-11-01T12:60:00",
            "2026-11-01T12:00:60",
            "2026-11-01T12:00:00.",
            "2026-11-01T12:00:00.1234567890",
            "2026-11-01T12:00:00+24:00",
            "2026-11-01T12:00:00+02",
            "2026-11-01T12:00",
            "2026-11-1T12:00:00",
            "2026-1-01T12:00:00",
            "2026-11-01T1:00:00",
            "2026-11-01T12:00:00+2:00",
            "2026-11-01T12:00:00:00",
            "2026-11-01-01T12:00:00",
            "+026-11-01T12:00:00",
            "1970-01-01T00:00:00",
        ] {
            assert_eq!(utc_seconds(text), None, "{text}");
        }
    }

    #[test]
    fn thumbnails_must_be_plain_https_locations() {
        assert_eq!(
            thumbnail("https://vz-a.b-cdn.net/id/thumbnail.jpg").as_deref(),
            Some("https://vz-a.b-cdn.net/id/thumbnail.jpg")
        );
        for text in [
            "http://vz-a.b-cdn.net/id/thumbnail.jpg",
            "https://user@vz-a.b-cdn.net/t.jpg",
            "https://:secret@vz-a.b-cdn.net/t.jpg",
            "https://vz-a.b-cdn.net/t.jpg#x",
            "javascript:alert(1)",
            "/relative.jpg",
        ] {
            assert_eq!(thumbnail(text), None, "{text}");
        }
        let prefix = "https://vz-a.b-cdn.net/";
        let longest = format!("{prefix}{}", "a".repeat(MAX_URL_BYTES - prefix.len()));
        assert_eq!(thumbnail(&longest).as_deref(), Some(longest.as_str()));
        assert_eq!(thumbnail(&format!("{longest}a")), None);
    }

    fn item(changes: serde_json::Value) -> Result<ListedVideo, Error> {
        let mut value = serde_json::json!({
            "videoLibraryId": 7, "guid": "12345678-1234-4234-8234-123456789abc",
            "title": "Lesson", "status": 4, "length": 60,
            "dateUploaded": "2026-11-01T12:00:00", "thumbnailUrl": "https://vz-a.b-cdn.net/t.jpg"
        });
        for (key, change) in changes.as_object().unwrap() {
            value[key] = change.clone();
        }
        let item: Item = serde_json::from_value(value).unwrap();
        item.checked(LibraryId::new(7).unwrap())
    }

    #[test]
    fn listed_items_are_bounded_and_identity_checked() {
        let video = item(serde_json::json!({})).unwrap();
        assert_eq!(video.processing, Processing::Ready);
        assert_eq!(video.created_at, Some(1_793_534_400));
        assert_eq!(
            video.thumbnail_url.as_deref(),
            Some("https://vz-a.b-cdn.net/t.jpg")
        );
        let accepted = [
            serde_json::json!({"title": "t".repeat(MAX_TITLE_BYTES)}),
            serde_json::json!({"length": 604_800}),
            serde_json::json!({"dateUploaded": null, "thumbnailUrl": null}),
        ];
        for changes in accepted {
            assert!(item(changes.clone()).is_ok(), "{changes}");
        }
        let empty = item(serde_json::json!({"dateUploaded": "", "thumbnailUrl": ""})).unwrap();
        assert_eq!((empty.created_at, empty.thumbnail_url), (None, None));
        for changes in [
            serde_json::json!({"videoLibraryId": 8}),
            serde_json::json!({"title": ""}),
            serde_json::json!({"title": "t".repeat(MAX_TITLE_BYTES + 1)}),
            serde_json::json!({"length": 604_801}),
            serde_json::json!({"status": 9}),
            serde_json::json!({"dateUploaded": "yesterday"}),
            serde_json::json!({"thumbnailUrl": "http://vz-a.b-cdn.net/t.jpg"}),
        ] {
            assert_eq!(item(changes.clone()), Err(Error::Protocol), "{changes}");
        }
    }
}
