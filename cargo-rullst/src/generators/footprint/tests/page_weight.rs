//! `--page-weight` against a loopback server whose page references
//! stylesheets, a script, images, a font, a redirect and third-party URLs,
//! each with its own caching headers.
use super::super::page::{self, PageWeight, Resource, Visit};
use super::support::{Request, TestServer, response};
use std::sync::Arc;

/// A measured page weight, for the report tests.
pub(in crate::generators::footprint) fn sample() -> PageWeight {
    PageWeight {
        status: "measured",
        reason: None,
        method: Some(page::METHOD),
        path: Some("/".to_string()),
        accept_encoding: Some("zstd, br, gzip"),
        first_visit: Visit {
            bytes: Some(48_000),
            requests: Some(6),
            from_cache: None,
            not_modified: None,
        },
        repeat_visit: Visit {
            bytes: Some(1_200),
            requests: Some(2),
            from_cache: Some(4),
            not_modified: Some(1),
        },
        resources: vec![Resource {
            url: "/".to_string(),
            kind: "document",
            status: Some(200),
            content_type: Some("text/html".to_string()),
            content_encoding: Some("br".to_string()),
            bytes: Some(1_200),
            repeat: "fetched",
            repeat_bytes: Some(1_200),
            error: None,
        }],
        third_party_origins: vec!["https://cdn.example".to_string()],
        third_party_references: 1,
        unfetched: 0,
    }
}

const PAGE: &str = r#"<!doctype html><html><head>
<link rel="preconnect" href="https://fonts.example">
<link rel="stylesheet" href="/static/app.css">
<link rel="icon" href="/old-icon">
<script src="static/app.js#ignored"></script>
</head><body>
<img src="/static/logo.png">
<img src="https://cdn.example/x.png">
<img src="data:image/png;base64,AAAA">
<img src="/static/logo.png">
</body></html>"#;

const STYLESHEET: &str = "@import \"theme.css\";\nbody { background: url(/static/bg.png) }\n@font-face { src: url(/static/f.woff2) format(\"woff2\"), url(/static/f.woff) }\n";

fn page_server(request: &Request) -> Option<Vec<u8>> {
    let accepts_br = request
        .header("accept-encoding")
        .is_some_and(|accept| accept.contains("br"));
    match request.path.as_str() {
        "/" if request.header("if-none-match") == Some("\"page1\"") => {
            response("304 Not Modified", "etag: \"page1\"\r\n", b"")
        }
        "/" if accepts_br => response(
            "200 OK",
            "content-type: text/html\r\ncontent-encoding: br\r\ncache-control: no-cache\r\netag: \"page1\"\r\n",
            b"COMPRESSED-HTML",
        ),
        "/" => response("200 OK", "content-type: text/html\r\n", PAGE.as_bytes()),
        "/static/app.css" => response(
            "200 OK",
            "content-type: text/css\r\ncache-control: public, max-age=31536000, immutable\r\n",
            STYLESHEET.as_bytes(),
        ),
        "/static/theme.css" => response("200 OK", "content-type: text/css\r\n", b"a{}"),
        "/static/app.js" if request.header("if-modified-since").is_some() => {
            response("304 Not Modified", "", b"")
        }
        "/static/app.js" => response(
            "200 OK",
            "cache-control: max-age=0\r\nlast-modified: Thu, 01 Jan 2026 00:00:00 GMT\r\n",
            b"console.log(1)",
        ),
        "/static/logo.png" => response("200 OK", "cache-control: max-age=600\r\n", &[7; 100]),
        "/static/bg.png" => response("200 OK", "cache-control: no-store\r\n", &[7; 50]),
        "/static/f.woff2" => response("200 OK", "cache-control: max-age=600\r\n", &[7; 20]),
        "/old-icon" => response(
            "301 Moved Permanently",
            "location: /static/icon.svg\r\ncache-control: max-age=3600\r\n",
            b"",
        ),
        "/static/icon.svg" => response("200 OK", "cache-control: max-age=600\r\n", &[7; 10]),
        _ => response("404 Not Found", "", b"missing"),
    }
}

/// A resource's URL, kind, status, bytes and repeat-visit outcome.
type Row<'a> = (&'a str, &'a str, Option<u16>, Option<u64>, &'a str);

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

fn weigh(server: &TestServer, path: &str) -> PageWeight {
    let client = super::super::load::client().unwrap();
    block_on(page::measure(&client, &server.base, path))
}

#[test]
fn the_first_and_repeat_visits_follow_the_page_and_its_cache_headers() {
    let server = TestServer::with(Arc::new(page_server));
    let weight = weigh(&server, "/");
    assert_eq!(weight.status, "measured", "{weight:?}");
    let urls: Vec<Row<'_>> = weight
        .resources
        .iter()
        .map(|resource| {
            (
                resource.url.as_str(),
                resource.kind,
                resource.status,
                resource.bytes,
                resource.repeat,
            )
        })
        .collect();
    let stylesheet = STYLESHEET.len() as u64;
    assert_eq!(
        urls,
        [
            ("/", "document", Some(200), Some(15), "not_modified"),
            (
                "/static/app.css",
                "stylesheet",
                Some(200),
                Some(stylesheet),
                "cache"
            ),
            ("/old-icon", "image", Some(301), Some(0), "cache"),
            (
                "/static/app.js",
                "script",
                Some(200),
                Some(14),
                "not_modified"
            ),
            ("/static/logo.png", "image", Some(200), Some(100), "cache"),
            (
                "/static/theme.css",
                "stylesheet",
                Some(200),
                Some(3),
                "fetched"
            ),
            ("/static/bg.png", "image", Some(200), Some(50), "fetched"),
            ("/static/f.woff2", "font", Some(200), Some(20), "cache"),
            ("/static/icon.svg", "image", Some(200), Some(10), "cache"),
        ]
    );
    assert_eq!(weight.resources[0].content_encoding.as_deref(), Some("br"));
    assert_eq!(weight.resources[1].content_encoding, None);
    assert_eq!(
        weight.resources[1].content_type.as_deref(),
        Some("text/css")
    );
    assert_eq!(
        weight.first_visit,
        Visit {
            bytes: Some(15 + stylesheet + 14 + 100 + 3 + 50 + 20 + 10),
            requests: Some(9),
            from_cache: None,
            not_modified: None,
        }
    );
    assert_eq!(
        weight.repeat_visit,
        Visit {
            bytes: Some(53),
            requests: Some(4),
            from_cache: Some(5),
            not_modified: Some(2),
        }
    );
    assert_eq!(
        weight.third_party_origins,
        ["https://cdn.example", "https://fonts.example"]
    );
    assert_eq!((weight.third_party_references, weight.unfetched), (1, 0));
    assert_eq!(weight.path.as_deref(), Some("/"));
    assert_eq!(weight.accept_encoding, Some("zstd, br, gzip"));

    // The compressed page was read again without compression for its
    // references, uncounted; nothing left the loopback origin.
    let requests = server.requests();
    let page_gets: Vec<Option<&str>> = requests
        .iter()
        .filter(|request| request.path == "/")
        .map(|request| request.header("accept-encoding"))
        .collect();
    assert_eq!(
        page_gets,
        [
            Some("zstd, br, gzip"),
            Some("identity"),
            Some("zstd, br, gzip")
        ]
    );
    // Only the first source of the @font-face list is loaded.
    assert!(
        !requests
            .iter()
            .any(|request| request.path == "/static/f.woff")
    );
}

#[test]
fn the_number_of_resources_is_bounded() {
    let images: String = (0..250)
        .map(|index| format!("<img src=\"/i/{index}.png\">"))
        .collect();
    let page = format!("<html><body>{images}</body></html>");
    let server = TestServer::with(Arc::new(move |request: &Request| {
        if request.path == "/" {
            response("200 OK", "content-type: text/html\r\n", page.as_bytes())
        } else {
            response("200 OK", "", b"x")
        }
    }));
    let weight = weigh(&server, "/");
    assert_eq!(weight.resources.len(), page::MAX_RESOURCES);
    assert_eq!(weight.first_visit.requests, Some(200));
    assert_eq!(weight.unfetched, 51);
}

#[test]
fn a_page_that_cannot_be_fetched_is_not_measured() {
    let server = TestServer::start();
    let weight = weigh(&server, "/hangup");
    assert_eq!(weight.status, "not_measured");
    assert!(
        weight
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("GET /hangup failed: ")),
        "{weight:?}"
    );
    assert_eq!(weight.first_visit, Visit::default());

    // A page without references is a single request.
    let weight = weigh(&server, "/target");
    assert_eq!(weight.status, "measured");
    assert_eq!(weight.first_visit.requests, Some(1));
    assert_eq!(weight.first_visit.bytes, Some(2));
}
