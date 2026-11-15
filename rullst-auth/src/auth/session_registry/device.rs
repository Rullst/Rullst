//! The minimal device summary of a session: a browser family and a platform
//! family read from the sign-in request's `User-Agent`.
//!
//! Only the two families are kept, never the header itself, versions, the
//! IP address or any other fingerprinting detail. The summary helps an
//! account recognise its own devices; it is not verified device identity
//! (any client can send any `User-Agent`).

use axum::http::{HeaderMap, header::USER_AGENT};
use std::fmt;

/// Bytes of a `User-Agent` that are examined; the rest is ignored.
const MAX_USER_AGENT_BYTES: usize = 512;

/// The browser family of a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Browser {
    Chrome,
    Edge,
    Firefox,
    Safari,
    Opera,
    SamsungInternet,
    /// No `User-Agent`, or one naming no known browser.
    Unknown,
}

/// The platform family of a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Platform {
    Windows,
    MacOs,
    Ios,
    Android,
    ChromeOs,
    Linux,
    /// No `User-Agent`, or one naming no known platform.
    Unknown,
}

/// A browser and platform family, shown as "Firefox on Linux".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceSummary {
    browser: Browser,
    platform: Platform,
}

/// Browser markers in the order they are tried: Chromium-based browsers
/// also say `Chrome/` and Safari, so the specific ones come first.
const BROWSERS: [(&str, Browser); 9] = [
    ("Edg/", Browser::Edge),
    ("EdgA/", Browser::Edge),
    ("EdgiOS/", Browser::Edge),
    ("OPR/", Browser::Opera),
    ("SamsungBrowser/", Browser::SamsungInternet),
    ("Firefox/", Browser::Firefox),
    ("FxiOS/", Browser::Firefox),
    ("Chrome/", Browser::Chrome),
    ("CriOS/", Browser::Chrome),
];

/// Platform markers in the order they are tried: Android and ChromeOS also
/// say `Linux`, and iOS devices say `like Mac OS X`.
const PLATFORMS: [(&str, Platform); 8] = [
    ("Windows", Platform::Windows),
    ("iPhone", Platform::Ios),
    ("iPad", Platform::Ios),
    ("iPod", Platform::Ios),
    ("Android", Platform::Android),
    ("CrOS", Platform::ChromeOs),
    ("Macintosh", Platform::MacOs),
    ("Linux", Platform::Linux),
];

impl DeviceSummary {
    /// A session whose device is not known.
    pub const fn unknown() -> Self {
        Self {
            browser: Browser::Unknown,
            platform: Platform::Unknown,
        }
    }

    /// A summary of these families.
    pub const fn new(browser: Browser, platform: Platform) -> Self {
        Self { browser, platform }
    }

    /// The families named by a `User-Agent` header value. Only its first
    /// 512 bytes are examined.
    pub fn from_user_agent(user_agent: &str) -> Self {
        let examined = &user_agent.as_bytes()[..user_agent.len().min(MAX_USER_AGENT_BYTES)];
        let names = |marker: &str| {
            examined
                .windows(marker.len())
                .any(|window| window == marker.as_bytes())
        };
        let browser = BROWSERS
            .iter()
            .find(|(marker, _)| names(marker))
            .map(|(_, browser)| *browser)
            .unwrap_or(if names("Safari/") && names("Version/") {
                Browser::Safari
            } else {
                Browser::Unknown
            });
        let platform = PLATFORMS
            .iter()
            .find(|(marker, _)| names(marker))
            .map_or(Platform::Unknown, |(_, platform)| *platform);
        Self { browser, platform }
    }

    /// The families named by the request's `User-Agent`; unknown when the
    /// header is absent or not visible ASCII.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        headers
            .get(USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map_or(Self::unknown(), Self::from_user_agent)
    }

    /// The browser family.
    pub fn browser(&self) -> Browser {
        self.browser
    }

    /// The platform family.
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// The stored codes of the two families.
    pub(super) fn codes(&self) -> (&'static str, &'static str) {
        (self.browser.code(), self.platform.code())
    }

    /// The summary of stored codes; a code this version does not know reads
    /// as unknown, so a newer writer never breaks an older reader.
    pub(super) fn from_codes(browser: &str, platform: &str) -> Self {
        Self {
            browser: Browser::from_code(browser),
            platform: Platform::from_code(platform),
        }
    }
}

impl Browser {
    /// The display name ("Firefox"); "Unknown browser" for `Unknown`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Chrome => "Chrome",
            Self::Edge => "Edge",
            Self::Firefox => "Firefox",
            Self::Safari => "Safari",
            Self::Opera => "Opera",
            Self::SamsungInternet => "Samsung Internet",
            Self::Unknown => "Unknown browser",
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Edge => "edge",
            Self::Firefox => "firefox",
            Self::Safari => "safari",
            Self::Opera => "opera",
            Self::SamsungInternet => "samsung",
            Self::Unknown => "unknown",
        }
    }

    fn from_code(code: &str) -> Self {
        match code {
            "chrome" => Self::Chrome,
            "edge" => Self::Edge,
            "firefox" => Self::Firefox,
            "safari" => Self::Safari,
            "opera" => Self::Opera,
            "samsung" => Self::SamsungInternet,
            _ => Self::Unknown,
        }
    }
}

impl Platform {
    /// The display name ("macOS"); "unknown device" for `Unknown`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::Ios => "iOS",
            Self::Android => "Android",
            Self::ChromeOs => "ChromeOS",
            Self::Linux => "Linux",
            Self::Unknown => "unknown device",
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::MacOs => "macos",
            Self::Ios => "ios",
            Self::Android => "android",
            Self::ChromeOs => "chromeos",
            Self::Linux => "linux",
            Self::Unknown => "unknown",
        }
    }

    fn from_code(code: &str) -> Self {
        match code {
            "windows" => Self::Windows,
            "macos" => Self::MacOs,
            "ios" => Self::Ios,
            "android" => Self::Android,
            "chromeos" => Self::ChromeOs,
            "linux" => Self::Linux,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for DeviceSummary {
    /// "Firefox on Linux", "Firefox", "Unknown browser on Android" or
    /// "Unknown device".
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.browser, self.platform) {
            (Browser::Unknown, Platform::Unknown) => formatter.write_str("Unknown device"),
            (browser, Platform::Unknown) => formatter.write_str(browser.name()),
            (browser, platform) => write!(formatter, "{} on {}", browser.name(), platform.name()),
        }
    }
}

#[cfg(test)]
#[path = "device_tests.rs"]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;
