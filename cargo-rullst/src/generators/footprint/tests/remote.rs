//! A published site's report (page weight only) and the bounds every page
//! weighing keeps: redirects in a row, the size of one response and time.
use super::super::page::{self, PageWeight};
use super::support::{TestServer, response};
use super::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

fn weigh_until(server: &TestServer, path: &str, deadline: Instant) -> PageWeight {
    let client = super::super::load::client().unwrap();
    block_on(page::measure_until(&client, &server.base, path, deadline))
}

fn weigh(server: &TestServer, path: &str) -> PageWeight {
    weigh_until(server, path, Instant::now() + Duration::from_secs(60))
}

#[test]
fn a_published_sites_report_names_what_was_not_measured() {
    let reason = super::super::subject::REMOTE;
    let mut report = sample(false, None);
    report.target = Target {
        mode: "remote_url",
        url: Some("https://app.example/".to_string()),
        pid: None,
        process_lookup: reason.to_string(),
        executable: None,
        environment: None,
    };
    report.measured = Measured::unmeasured(reason);
    report.notes = report::REMOTE_NOTES;
    let text = render::render(&report, Style { color: false });
    for expected in [
        "Mode          published site (--url https://), page weight only",
        "URL           https://app.example/",
        "Process       NOT LOOKED UP",
        "Only the page weight was measured",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    assert!(!text.contains("NOT FOUND"), "{text}");
    for metric in [
        &report.measured.binary_size_bytes,
        &report.measured.image_size_bytes,
        &report.measured.idle_rss_bytes,
    ] {
        assert_eq!(
            (metric.status, metric.unit, metric.reason.as_deref()),
            ("not_measured", "bytes", Some(reason))
        );
    }
    for metric in [
        &report.measured.dependency_count,
        &report.measured.compiled_dependency_count,
    ] {
        assert_eq!(
            (metric.status, metric.unit, metric.reason.as_deref()),
            ("not_measured", "packages", Some(reason))
        );
    }
    // A failure keeps the published site's own wording.
    report.failure = Some(super::super::failure::Failure::remote(
        "https://app.example/",
        "GET / failed",
    ));
    let text = render::render(&report, Style { color: false });
    assert!(text.contains("Process       NOT LOOKED UP"), "{text}");
}

#[test]
fn redirects_are_followed_five_in_a_row_at_most() {
    // `/hop/N` redirects to `/hop/N+1`; the page itself redirects to `/hop/1`.
    let server = TestServer::with(Arc::new(|request| {
        let next = match request.path.as_str() {
            "/" => 1,
            path => path.trim_start_matches("/hop/").parse::<u32>().unwrap() + 1,
        };
        response("302 Found", &format!("location: /hop/{next}\r\n"), b"")
    }));
    let weight = weigh(&server, "/");
    assert_eq!(weight.status, "measured", "{weight:?}");
    // The page and five redirects were fetched; the sixth was not followed.
    assert_eq!(
        server.paths(),
        ["/", "/hop/1", "/hop/2", "/hop/3", "/hop/4", "/hop/5"]
            .into_iter()
            .chain(["/", "/hop/1", "/hop/2", "/hop/3", "/hop/4", "/hop/5"])
            .collect::<Vec<_>>()
    );
    assert_eq!(weight.first_visit.requests, Some(6));
    assert_eq!(weight.unfetched, 1);
}

#[test]
fn a_response_over_32_mib_is_an_error_left_out_of_the_totals() {
    let server = TestServer::with(Arc::new(|request| match request.path.as_str() {
        "/" => response(
            "200 OK",
            "",
            br#"<img src="/huge.png"><img src="/small.png">"#,
        ),
        "/huge.png" => response("200 OK", "", &vec![b'x'; 32 * 1024 * 1024 + 1]),
        "/exact.png" => response("200 OK", "", &vec![b'x'; 32 * 1024 * 1024]),
        _ => response("200 OK", "", b"ok"),
    }));
    let weight = weigh(&server, "/");
    assert_eq!(weight.status, "measured", "{weight:?}");
    let huge = &weight.resources[1];
    assert_eq!(huge.url, "/huge.png");
    assert_eq!(huge.bytes, None);
    assert_eq!(
        huge.error.as_deref(),
        Some("the response exceeds 32 MiB; footprint stopped reading it")
    );
    // The page (43 bytes) and the small image (2) count; the huge one does not.
    assert_eq!(weight.first_visit.bytes, Some(43 + 2));
    // Exactly 32 MiB is still read; a page over the bound is not measured.
    assert_eq!(
        weigh(&server, "/exact.png").first_visit.bytes,
        Some(32 * 1024 * 1024)
    );
    let page = weigh(&server, "/huge.png");
    assert_eq!(page.status, "not_measured");
    assert_eq!(
        page.reason.as_deref(),
        Some("GET /huge.png failed: the response exceeds 32 MiB; footprint stopped reading it")
    );
}

#[test]
fn the_time_limit_leaves_the_rest_unfetched_and_unrevisited() {
    let server = TestServer::with(Arc::new(|request| match request.path.as_str() {
        "/" => response(
            "200 OK",
            "",
            br#"<link rel="stylesheet" href="/slow.css"><img src="/a.png">"#,
        ),
        "/slow.css" => {
            std::thread::sleep(Duration::from_millis(400));
            response("200 OK", "", b"body{}")
        }
        _ => response("200 OK", "", b"ok"),
    }));
    let weight = weigh_until(&server, "/", Instant::now() + Duration::from_millis(200));
    assert_eq!(weight.status, "measured", "{weight:?}");
    // The page and the slow stylesheet were fetched; the image was not, and
    // the repeat visit had no time left.
    assert_eq!(weight.first_visit.requests, Some(2));
    assert_eq!(weight.unfetched, 1);
    assert_eq!(weight.repeat_visit.requests, Some(0));
    for resource in &weight.resources {
        assert_eq!(resource.repeat, "error", "{resource:?}");
        assert_eq!(
            resource.error.as_deref(),
            Some("not revisited: the 180-second limit was reached")
        );
    }
    assert_eq!(server.paths(), ["/", "/slow.css"]);
    // A deadline already past sends nothing.
    let none = weigh_until(&server, "/", Instant::now());
    assert_eq!(none.status, "not_measured");
    assert_eq!(
        none.reason.as_deref(),
        Some("GET / was not sent: the 180-second limit was reached")
    );
}
