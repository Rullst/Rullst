use super::*;
use axum::http::HeaderValue;

const CHROME_WINDOWS: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const EDGE_WINDOWS: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0";
const FIREFOX_LINUX: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0";
const SAFARI_MAC: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15";
const SAFARI_IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1";
const CHROME_IPAD: &str = "Mozilla/5.0 (iPad; CPU OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/131.0.6778.73 Mobile/15E148 Safari/604.1";
const FIREFOX_IPOD: &str = "Mozilla/5.0 (iPod touch; CPU iPhone OS 15_8 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) FxiOS/133.0 Mobile/15E148 Safari/605.1.15";
const SAMSUNG_ANDROID: &str = "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/26.0 Chrome/122.0.0.0 Mobile Safari/537.36";
const EDGE_ANDROID: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36 EdgA/131.0.0.0";
const EDGE_IOS: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) EdgiOS/131.0.2903.92 Version/18.0 Mobile/15E148 Safari/604.1";
const OPERA_CHROMEOS: &str = "Mozilla/5.0 (X11; CrOS x86_64 14541.0.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 OPR/115.0.0.0";

#[test]
fn user_agents_map_to_their_browser_and_platform_families() {
    for (user_agent, browser, platform, shown) in [
        (
            CHROME_WINDOWS,
            Browser::Chrome,
            Platform::Windows,
            "Chrome on Windows",
        ),
        (
            EDGE_WINDOWS,
            Browser::Edge,
            Platform::Windows,
            "Edge on Windows",
        ),
        (
            FIREFOX_LINUX,
            Browser::Firefox,
            Platform::Linux,
            "Firefox on Linux",
        ),
        (
            SAFARI_MAC,
            Browser::Safari,
            Platform::MacOs,
            "Safari on macOS",
        ),
        (
            SAFARI_IPHONE,
            Browser::Safari,
            Platform::Ios,
            "Safari on iOS",
        ),
        (CHROME_IPAD, Browser::Chrome, Platform::Ios, "Chrome on iOS"),
        (
            FIREFOX_IPOD,
            Browser::Firefox,
            Platform::Ios,
            "Firefox on iOS",
        ),
        (
            SAMSUNG_ANDROID,
            Browser::SamsungInternet,
            Platform::Android,
            "Samsung Internet on Android",
        ),
        (
            EDGE_ANDROID,
            Browser::Edge,
            Platform::Android,
            "Edge on Android",
        ),
        (EDGE_IOS, Browser::Edge, Platform::Ios, "Edge on iOS"),
        (
            OPERA_CHROMEOS,
            Browser::Opera,
            Platform::ChromeOs,
            "Opera on ChromeOS",
        ),
    ] {
        let device = DeviceSummary::from_user_agent(user_agent);
        assert_eq!(
            (device.browser(), device.platform()),
            (browser, platform),
            "{user_agent}"
        );
        assert_eq!(device.to_string(), shown);
        // Stored as two codes and read back unchanged.
        let (browser_code, platform_code) = device.codes();
        assert_eq!(
            DeviceSummary::from_codes(browser_code, platform_code),
            device
        );
    }
}

#[test]
fn unknown_families_read_as_unknown() {
    assert_eq!(DeviceSummary::from_user_agent(""), DeviceSummary::unknown());
    assert_eq!(
        DeviceSummary::from_user_agent("curl/8.5.0"),
        DeviceSummary::unknown()
    );
    assert_eq!(DeviceSummary::unknown().to_string(), "Unknown device");
    // Safari needs both `Version/` and `Safari/`.
    let webview = DeviceSummary::from_user_agent(
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 Safari/604.1",
    );
    assert_eq!(webview, DeviceSummary::new(Browser::Unknown, Platform::Ios));
    assert_eq!(webview.to_string(), "Unknown browser on iOS");
    assert_eq!(
        DeviceSummary::from_user_agent("Version/1.0 Mozilla/5.0").browser(),
        Browser::Unknown
    );
    let firefox = DeviceSummary::new(Browser::Firefox, Platform::Unknown);
    assert_eq!(firefox.to_string(), "Firefox");
    // Codes a newer version might write read as unknown.
    assert_eq!(
        DeviceSummary::from_codes("netscape", "beos"),
        DeviceSummary::unknown()
    );
}

#[test]
fn every_family_has_a_distinct_code_and_name() {
    let browsers = [
        Browser::Chrome,
        Browser::Edge,
        Browser::Firefox,
        Browser::Safari,
        Browser::Opera,
        Browser::SamsungInternet,
        Browser::Unknown,
    ];
    let platforms = [
        Platform::Windows,
        Platform::MacOs,
        Platform::Ios,
        Platform::Android,
        Platform::ChromeOs,
        Platform::Linux,
        Platform::Unknown,
    ];
    assert_eq!(
        browsers.map(|browser| browser.code()),
        [
            "chrome", "edge", "firefox", "safari", "opera", "samsung", "unknown"
        ]
    );
    assert_eq!(
        browsers.map(Browser::name),
        [
            "Chrome",
            "Edge",
            "Firefox",
            "Safari",
            "Opera",
            "Samsung Internet",
            "Unknown browser"
        ]
    );
    assert_eq!(
        platforms.map(|platform| platform.code()),
        [
            "windows", "macos", "ios", "android", "chromeos", "linux", "unknown"
        ]
    );
    assert_eq!(
        platforms.map(Platform::name),
        [
            "Windows",
            "macOS",
            "iOS",
            "Android",
            "ChromeOS",
            "Linux",
            "unknown device"
        ]
    );
    for browser in browsers {
        assert_eq!(Browser::from_code(browser.code()), browser);
    }
    for platform in platforms {
        assert_eq!(Platform::from_code(platform.code()), platform);
    }
}

#[test]
fn only_the_first_512_bytes_are_examined() {
    let padded = format!("{}{FIREFOX_LINUX}", "x".repeat(MAX_USER_AGENT_BYTES));
    assert_eq!(
        DeviceSummary::from_user_agent(&padded),
        DeviceSummary::unknown()
    );
    // A marker ending exactly at byte 512 is seen.
    let edge = format!("{}Linux", "x".repeat(MAX_USER_AGENT_BYTES - 5));
    assert_eq!(edge.len(), 512);
    assert_eq!(
        DeviceSummary::from_user_agent(&edge).platform(),
        Platform::Linux
    );
    // A cut inside a multi-byte character is harmless.
    let multibyte = format!("{}é Firefox/1", "x".repeat(MAX_USER_AGENT_BYTES - 1));
    assert_eq!(
        DeviceSummary::from_user_agent(&multibyte),
        DeviceSummary::unknown()
    );
}

#[test]
fn headers_without_a_readable_user_agent_are_unknown() {
    let mut headers = HeaderMap::new();
    assert_eq!(
        DeviceSummary::from_headers(&headers),
        DeviceSummary::unknown()
    );
    headers.insert(USER_AGENT, HeaderValue::from_static(FIREFOX_LINUX));
    assert_eq!(
        DeviceSummary::from_headers(&headers),
        DeviceSummary::new(Browser::Firefox, Platform::Linux)
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_bytes(b"Firefox/1 \xff").unwrap(),
    );
    assert_eq!(
        DeviceSummary::from_headers(&headers),
        DeviceSummary::unknown()
    );
}
