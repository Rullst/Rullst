//! Provider library listing (v13): one page of the videos a library holds,
//! including videos created outside the application.

use crate::{MediaError as Error, Processing, VideoId};

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
