//! Provider library listing (v13): one page of the videos a library holds,
//! including videos created outside the application.

use crate::contracts::checked_time;
use crate::{MediaError as Error, Processing, ProviderMode, VideoId};

/// Longest listed title, in bytes.
const MAX_TITLE_BYTES: usize = 1_024;
/// Longest video duration accepted, one week in seconds.
const MAX_LENGTH_SECONDS: u32 = 604_800;

/// Order of a [`ListQuery`] page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ListOrder {
    /// By upload date, in the provider's date order (Bunny `orderBy=date`).
    #[default]
    Date,
    /// By title (Bunny `orderBy=title`).
    Title,
}

/// A bounded library page request: page number, page size, an optional
/// case-insensitive title search and the order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListQuery {
    page: u32,
    per_page: u32,
    search: Option<String>,
    order: ListOrder,
}

impl ListQuery {
    /// Smallest page size; Bunny raises smaller sizes to 10.
    pub const MIN_PER_PAGE: u32 = 10;
    /// Largest page size, which keeps a provider page within 1 MiB.
    pub const MAX_PER_PAGE: u32 = 100;
    /// Highest page number accepted.
    pub const MAX_PAGE: u32 = 100_000;
    /// Longest search term, in bytes.
    pub const MAX_SEARCH_BYTES: usize = 100;

    /// Page `page` (from 1) of `per_page` videos, by date, without search.
    pub fn new(page: u32, per_page: u32) -> Result<Self, Error> {
        if !(1..=Self::MAX_PAGE).contains(&page)
            || !(Self::MIN_PER_PAGE..=Self::MAX_PER_PAGE).contains(&per_page)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            page,
            per_page,
            search: None,
            order: ListOrder::Date,
        })
    }

    /// Keeps only videos whose title contains `term`. A blank term clears the
    /// search; a term longer than [`MAX_SEARCH_BYTES`](Self::MAX_SEARCH_BYTES)
    /// or holding a control character is refused.
    pub fn with_search(mut self, term: impl Into<String>) -> Result<Self, Error> {
        let term = term.into();
        let term = term.trim();
        if term.len() > Self::MAX_SEARCH_BYTES || term.chars().any(char::is_control) {
            return Err(Error::InvalidInput);
        }
        self.search = (!term.is_empty()).then(|| term.to_owned());
        Ok(self)
    }

    pub fn with_order(mut self, order: ListOrder) -> Self {
        self.order = order;
        self
    }

    pub fn page(&self) -> u32 {
        self.page
    }
    pub fn per_page(&self) -> u32 {
        self.per_page
    }
    pub fn search(&self) -> Option<&str> {
        self.search.as_deref()
    }
    pub fn order(&self) -> ListOrder {
        self.order
    }
}

/// One listed video after identity, status and size validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListedVideo {
    pub id: VideoId,
    /// The provider title, at most 1024 bytes. Escape it like any untrusted
    /// text: videos created elsewhere may hold any title.
    pub title: String,
    /// Duration in seconds; 0 until processing measures it.
    pub length_seconds: u32,
    pub processing: Processing,
    /// Upload time in UTC Unix seconds, when the provider reports one.
    pub created_at: Option<i64>,
    /// The provider's `https` CDN location of the thumbnail. A location, not
    /// a playback grant: with CDN token authentication it needs a signature.
    pub thumbnail_url: Option<String>,
}

impl ListedVideo {
    /// A listed video read by an application-owned
    /// [`VideoProvider`](crate::VideoProvider) from its service.
    /// Since 13.0.0-alpha.5.
    ///
    /// `title` is 1 to 1024 bytes (any text: escape it when rendering) and
    /// `length_seconds` at most one week (604,800); anything else is
    /// [`MediaError::InvalidInput`](Error::InvalidInput). Add the upload time
    /// and thumbnail with [`with_created_at`](Self::with_created_at) and
    /// [`with_thumbnail_url`](Self::with_thumbnail_url).
    pub fn new(
        id: VideoId,
        title: impl Into<String>,
        length_seconds: u32,
        processing: Processing,
    ) -> Result<Self, Error> {
        let title = title.into();
        if title.is_empty() || title.len() > MAX_TITLE_BYTES || length_seconds > MAX_LENGTH_SECONDS
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            id,
            title,
            length_seconds,
            processing,
            created_at: None,
            thumbnail_url: None,
        })
    }

    /// The upload time in UTC Unix seconds, between 1970 and 9999.
    /// Since 13.0.0-alpha.5.
    pub fn with_created_at(mut self, created_at: i64) -> Result<Self, Error> {
        self.created_at = Some(checked_time(created_at).map_err(|_| Error::InvalidInput)?);
        Ok(self)
    }

    /// The thumbnail's absolute `https://` location: at most 2 KiB, without
    /// credentials, fragment, spaces or control characters. Since
    /// 13.0.0-alpha.5.
    pub fn with_thumbnail_url(mut self, url: impl Into<String>) -> Result<Self, Error> {
        let url = url.into();
        if !crate::grant::url_valid(&url, ProviderMode::Offline) {
            return Err(Error::InvalidInput);
        }
        self.thumbnail_url = Some(url);
        Ok(self)
    }
}

/// One page of a provider library.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VideoPage {
    pub items: Vec<ListedVideo>,
    pub page: u32,
    pub per_page: u32,
    /// Matching videos across all pages.
    pub total: u64,
}

impl VideoPage {
    /// The page `query` asked for, built by an application-owned
    /// [`VideoProvider`](crate::VideoProvider). Since 13.0.0-alpha.5.
    ///
    /// The page number and size are the query's. `items` holds at most
    /// `query.per_page()` videos and `total` (matching videos across all
    /// pages) is at least their number; anything else is
    /// [`MediaError::InvalidInput`](Error::InvalidInput).
    pub fn new(query: &ListQuery, items: Vec<ListedVideo>, total: u64) -> Result<Self, Error> {
        let count = items.len();
        if count > query.per_page() as usize || total < count as u64 {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            items,
            page: query.page(),
            per_page: query.per_page(),
            total,
        })
    }

    /// Whether a later page holds more matching videos.
    pub fn has_more(&self) -> bool {
        u64::from(self.page).saturating_mul(u64::from(self.per_page)) < self.total
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn page(page: u32, per_page: u32, total: u64) -> VideoPage {
        VideoPage {
            items: Vec::new(),
            page,
            per_page,
            total,
        }
    }

    #[test]
    fn more_pages_follow_only_while_matches_remain() {
        assert!(!page(1, 10, 10).has_more());
        assert!(page(1, 10, 11).has_more());
        assert!(!page(2, 10, 20).has_more());
        assert!(page(2, 10, 21).has_more());
        assert!(!page(3, 10, 0).has_more());
        assert!(page(u32::MAX, u32::MAX, u64::MAX).has_more());
    }

    fn listed(title: &str, length: u32) -> Result<ListedVideo, Error> {
        let id = VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap();
        ListedVideo::new(id, title, length, Processing::Ready)
    }

    #[test]
    fn listed_videos_validate_title_length_time_and_thumbnail() {
        let video = listed("Lesson", 754).unwrap();
        assert_eq!(
            (video.title.as_str(), video.length_seconds, video.processing),
            ("Lesson", 754, Processing::Ready)
        );
        assert_eq!(
            (video.created_at, video.thumbnail_url.as_deref()),
            (None, None)
        );
        let longest = "t".repeat(MAX_TITLE_BYTES);
        assert!(listed(&longest, MAX_LENGTH_SECONDS).is_ok());
        assert_eq!(listed("", 1), Err(Error::InvalidInput));
        assert_eq!(listed(&format!("{longest}t"), 1), Err(Error::InvalidInput));
        assert_eq!(
            listed("Lesson", MAX_LENGTH_SECONDS + 1),
            Err(Error::InvalidInput)
        );

        let dated = video.clone().with_created_at(1_793_534_400).unwrap();
        assert_eq!(dated.created_at, Some(1_793_534_400));
        assert_eq!(video.clone().with_created_at(0), Err(Error::InvalidInput));
        let thumb = "https://cdn.example/v/thumbnail.jpg";
        let shown = video.clone().with_thumbnail_url(thumb).unwrap();
        assert_eq!(shown.thumbnail_url.as_deref(), Some(thumb));
        for url in [
            "http://cdn.example/t.jpg",
            "https://u:p@cdn.example/t.jpg",
            "https://cdn.example/t.jpg#x",
        ] {
            assert_eq!(
                video.clone().with_thumbnail_url(url),
                Err(Error::InvalidInput),
                "{url}"
            );
        }
    }

    #[test]
    fn pages_echo_their_query_and_bound_their_items() {
        let query = ListQuery::new(2, 10).unwrap();
        let ten = vec![listed("Lesson", 1).unwrap(); 10];
        let full = VideoPage::new(&query, ten.clone(), 10).unwrap();
        assert_eq!(
            (full.page, full.per_page, full.total, full.items.len()),
            (2, 10, 10, 10)
        );
        let mut eleven = ten.clone();
        eleven.push(listed("Lesson", 1).unwrap());
        assert_eq!(VideoPage::new(&query, eleven, 20), Err(Error::InvalidInput));
        assert_eq!(VideoPage::new(&query, ten, 9), Err(Error::InvalidInput));
        let empty = VideoPage::new(&query, Vec::new(), 0).unwrap();
        assert!(empty.items.is_empty());
    }

    #[test]
    fn query_options_are_kept() {
        let query = ListQuery::new(3, 25).unwrap().with_order(ListOrder::Title);
        assert_eq!(query.order(), ListOrder::Title);
        assert_eq!(
            (query.page(), query.per_page(), query.search()),
            (3, 25, None)
        );
        let query = ListQuery::new(1, 10).unwrap();
        assert_eq!((query.page(), query.per_page()), (1, 10));
        assert_eq!(ListOrder::default(), ListOrder::Date);
    }
}
