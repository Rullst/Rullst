//! `--page-weight`: the bytes and requests of the `--path` page and the
//! same-origin resources it references, on a first visit (empty cache) and
//! on a repeat visit (the cache the first visit left, per [`cache`]).
//! Responses are counted as served, compressed when the app compresses
//! them, without HTTP headers. Third-party origins are listed, never
//! fetched, so every request stays on the validated loopback origin.
mod cache;
mod scan;

use cache::{CacheHeaders, Reuse, Validators};
use scan::{Kind, Reference};
use serde::Serialize;
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::time::Instant;

pub(super) const METHOD: &str = "GET of the page and of the same-origin stylesheets, scripts, fonts, images, icons and manifest it references (with the stylesheets' url() and @import), each once, with Accept-Encoding: zstd, br, gzip; bytes are response bodies as served, without headers. The repeat visit reuses a response while Cache-Control max-age (or Expires) keeps it fresh, revalidates it with If-None-Match or If-Modified-Since when it has a validator, and fetches it again otherwise.";
/// Same-origin responses fetched per visit, redirects included.
pub(super) const MAX_RESOURCES: usize = 200;
/// Bytes of a page or stylesheet read for its references.
const MAX_PARSE_BYTES: usize = 4 * 1024 * 1024;
/// Levels of `@import` followed from the page's stylesheets.
const MAX_STYLESHEET_DEPTH: usize = 3;

/// The totals of one visit.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Visit {
    /// Response body bytes as served.
    pub bytes: Option<u64>,
    /// Requests sent, redirects and failed requests included.
    pub requests: Option<u64>,
    /// Repeat visit only: responses reused from the cache without a request.
    pub from_cache: Option<u64>,
    /// Repeat visit only: conditional requests answered `304 Not Modified`.
    pub not_modified: Option<u64>,
}

/// One same-origin response of the first visit and what the repeat visit
/// did with it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Resource {
    /// The path and query on the app's origin.
    pub url: String,
    /// `document`, `stylesheet`, `script`, `font`, `image`, `manifest` or `other`.
    pub kind: &'static str,
    pub status: Option<u16>,
    pub content_type: Option<String>,
    /// `null` when the response was not compressed.
    pub content_encoding: Option<String>,
    pub bytes: Option<u64>,
    /// `cache` (reused without a request), `not_modified` (a 304),
    /// `fetched` or `error`.
    pub repeat: &'static str,
    pub repeat_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct PageWeight {
    /// `measured`, `not_run` (without `--page-weight`) or `not_measured`.
    pub status: &'static str,
    pub reason: Option<String>,
    pub method: Option<&'static str>,
    pub path: Option<String>,
    pub accept_encoding: Option<&'static str>,
    pub first_visit: Visit,
    pub repeat_visit: Visit,
    pub resources: Vec<Resource>,
    /// Origins of other hosts the page references, sorted; never fetched.
    pub third_party_origins: Vec<String>,
    /// Distinct third-party URLs the page would load (connection hints such
    /// as `preconnect` only add their origin).
    pub third_party_references: u64,
    /// Same-origin references left out by the bound on resources.
    pub unfetched: u64,
}

impl PageWeight {
    /// No page weight: `not_run` without `--page-weight`, otherwise
    /// `not_measured`, with the reason.
    pub(super) fn unmeasured(status: &'static str, reason: impl Into<String>) -> Self {
        Self {
            status,
            reason: Some(reason.into()),
            method: None,
            path: None,
            accept_encoding: None,
            first_visit: Visit::default(),
            repeat_visit: Visit::default(),
            resources: Vec::new(),
            third_party_origins: Vec::new(),
            third_party_references: 0,
            unfetched: 0,
        }
    }
}

/// One response as the visits count it.
struct Response {
    status: u16,
    bytes: u64,
    content_type: Option<String>,
    content_encoding: Option<String>,
    cache: CacheHeaders,
    location: Option<String>,
    /// The body when asked for, at most [`MAX_PARSE_BYTES`].
    text: Option<String>,
    at: Instant,
}

fn header(response: &reqwest::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn get(
    client: &reqwest::Client,
    url: &reqwest::Url,
    accept_encoding: &str,
    validators: Option<&Validators>,
    keep: bool,
) -> Result<Response, String> {
    let mut request = client
        .get(url.clone())
        .header(reqwest::header::ACCEPT_ENCODING, accept_encoding);
    if let Some(validators) = validators {
        if let Some(etag) = &validators.etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(modified) = &validators.last_modified {
            request = request.header(reqwest::header::IF_MODIFIED_SINCE, modified);
        }
    }
    let mut response = request.send().await.map_err(|error| error.to_string())?;
    let at = Instant::now();
    let cache = CacheHeaders {
        cache_control: header(&response, "cache-control"),
        expires: header(&response, "expires"),
        date: header(&response, "date"),
        age: header(&response, "age"),
        etag: header(&response, "etag"),
        last_modified: header(&response, "last-modified"),
    };
    let content_encoding = header(&response, "content-encoding")
        .filter(|encoding| !encoding.eq_ignore_ascii_case("identity"));
    let (status, content_type, location) = (
        response.status().as_u16(),
        header(&response, "content-type"),
        header(&response, "location"),
    );
    let mut bytes = 0_u64;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        bytes = bytes.saturating_add(chunk.len() as u64);
        if keep {
            let room = MAX_PARSE_BYTES.saturating_sub(body.len());
            body.extend_from_slice(&chunk[..chunk.len().min(room)]);
        }
    }
    Ok(Response {
        status,
        bytes,
        content_type,
        content_encoding,
        cache,
        location,
        text: keep.then(|| String::from_utf8_lossy(&body).into_owned()),
        at,
    })
}

/// A first-visit response kept for the repeat visit.
struct Entry {
    url: reqwest::Url,
    resource: Resource,
    reuse: Option<(CacheHeaders, Instant)>,
}

/// The crawl of one page on one origin.
struct Crawl<'a> {
    client: &'a reqwest::Client,
    origin: reqwest::Url,
    seen: HashSet<String>,
    queue: VecDeque<(reqwest::Url, Kind, usize)>,
    entries: Vec<Entry>,
    third_party: BTreeSet<String>,
    third_party_urls: HashSet<String>,
    unfetched: u64,
}

impl Crawl<'_> {
    /// Queues `reference` as written in the document or stylesheet at `from`.
    fn reference(&mut self, from: &reqwest::Url, reference: &Reference, depth: usize) {
        let Ok(mut url) = from.join(&reference.url) else {
            return;
        };
        if !matches!(url.scheme(), "http" | "https") {
            return;
        }
        url.set_fragment(None);
        if url.origin() != self.origin.origin() {
            self.third_party.insert(url.origin().ascii_serialization());
            if reference.kind != Kind::Connection {
                self.third_party_urls.insert(url.to_string());
            }
            return;
        }
        if reference.kind != Kind::Connection && self.seen.insert(url.to_string()) {
            self.queue.push_back((url, reference.kind, depth));
        }
    }

    /// The text of a response to parse: its own body when it was not
    /// compressed, otherwise an uncompressed GET that is not counted.
    async fn text(&self, url: &reqwest::Url, response: &mut Response) -> Option<String> {
        if response.content_encoding.is_none() {
            return response.text.take();
        }
        get(self.client, url, "identity", None, true)
            .await
            .ok()
            .and_then(|response| response.text)
    }

    async fn visit(&mut self, url: reqwest::Url, kind: Kind, depth: usize) {
        let parse = (kind == Kind::Document && depth == 0)
            || (kind == Kind::Stylesheet && depth <= MAX_STYLESHEET_DEPTH);
        let path = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_string(),
        };
        let mut resource = Resource {
            url: path,
            kind: kind.as_str(),
            status: None,
            content_type: None,
            content_encoding: None,
            bytes: None,
            repeat: "fetched",
            repeat_bytes: None,
            error: None,
        };
        let mut response =
            match get(self.client, &url, super::load::ACCEPT_ENCODING, None, parse).await {
                Ok(response) => response,
                Err(error) => {
                    resource.error = Some(error);
                    self.entries.push(Entry {
                        url,
                        resource,
                        reuse: None,
                    });
                    return;
                }
            };
        resource.status = Some(response.status);
        resource.content_type = response.content_type.clone();
        resource.content_encoding = response.content_encoding.clone();
        resource.bytes = Some(response.bytes);
        if (300..400).contains(&response.status)
            && let Some(location) = response.location.clone()
        {
            let target = Reference {
                url: location,
                kind,
            };
            self.reference(&url, &target, depth);
        } else if parse
            && (200..300).contains(&response.status)
            && let Some(text) = self.text(&url, &mut response).await
        {
            if kind == Kind::Document {
                let html = response
                    .content_type
                    .as_deref()
                    .is_none_or(|content_type| content_type.contains("html"));
                if html {
                    let page = scan::html(&text);
                    let base = page
                        .base
                        .as_deref()
                        .and_then(|base| url.join(base).ok())
                        .unwrap_or_else(|| url.clone());
                    for reference in &page.references {
                        self.reference(&base, reference, depth + 1);
                    }
                }
            } else {
                for reference in scan::css(&text) {
                    self.reference(&url, &reference, depth + 1);
                }
            }
        }
        self.entries.push(Entry {
            url,
            resource,
            reuse: Some((response.cache, response.at)),
        });
    }

    /// The repeat visit of every first-visit response.
    async fn repeat(&mut self) -> Visit {
        let mut visit = Visit {
            bytes: Some(0),
            requests: Some(0),
            from_cache: Some(0),
            not_modified: Some(0),
        };
        let add = |total: &mut Option<u64>, amount: u64| {
            *total = Some(total.unwrap_or(0).saturating_add(amount));
        };
        for entry in &mut self.entries {
            let reuse = entry
                .reuse
                .as_ref()
                .map_or(Reuse::Refetch, |(headers, at)| {
                    cache::reuse(headers, at.elapsed().as_secs())
                });
            let validators = match reuse {
                Reuse::Fresh(_) => {
                    entry.resource.repeat = "cache";
                    entry.resource.repeat_bytes = Some(0);
                    add(&mut visit.from_cache, 1);
                    continue;
                }
                Reuse::Revalidate(validators) => Some(validators),
                Reuse::Refetch => None,
            };
            add(&mut visit.requests, 1);
            let accept = super::load::ACCEPT_ENCODING;
            match get(self.client, &entry.url, accept, validators.as_ref(), false).await {
                Ok(response) if response.status == 304 => {
                    entry.resource.repeat = "not_modified";
                    entry.resource.repeat_bytes = Some(response.bytes);
                    add(&mut visit.not_modified, 1);
                    add(&mut visit.bytes, response.bytes);
                }
                Ok(response) => {
                    entry.resource.repeat_bytes = Some(response.bytes);
                    add(&mut visit.bytes, response.bytes);
                }
                Err(error) => {
                    entry.resource.repeat = "error";
                    entry.resource.error.get_or_insert(error);
                }
            }
        }
        visit
    }
}

/// Measures the page at `path` on `base`, the app's loopback origin.
pub(super) async fn measure(
    client: &reqwest::Client,
    base: &reqwest::Url,
    path: &str,
) -> PageWeight {
    let start = super::target::request_url(base, path);
    let mut crawl = Crawl {
        client,
        origin: base.clone(),
        seen: HashSet::from([start.to_string()]),
        queue: VecDeque::from([(start, Kind::Document, 0)]),
        entries: Vec::new(),
        third_party: BTreeSet::new(),
        third_party_urls: HashSet::new(),
        unfetched: 0,
    };
    while let Some((url, kind, depth)) = crawl.queue.pop_front() {
        if crawl.entries.len() >= MAX_RESOURCES {
            crawl.unfetched = 1 + crawl.queue.len() as u64;
            break;
        }
        crawl.visit(url, kind, depth).await;
    }
    if let Some(Entry {
        resource: Resource {
            error: Some(error), ..
        },
        ..
    }) = crawl.entries.first()
    {
        return PageWeight::unmeasured("not_measured", format!("GET {path} failed: {error}"));
    }
    let first_visit = Visit {
        bytes: Some(
            crawl
                .entries
                .iter()
                .filter_map(|entry| entry.resource.bytes)
                .sum(),
        ),
        requests: Some(crawl.entries.len() as u64),
        from_cache: None,
        not_modified: None,
    };
    let repeat_visit = crawl.repeat().await;
    PageWeight {
        status: "measured",
        reason: None,
        method: Some(METHOD),
        path: Some(path.to_string()),
        accept_encoding: Some(super::load::ACCEPT_ENCODING),
        first_visit,
        repeat_visit,
        resources: crawl
            .entries
            .into_iter()
            .map(|entry| entry.resource)
            .collect(),
        third_party_origins: crawl.third_party.into_iter().collect(),
        third_party_references: crawl.third_party_urls.len() as u64,
        unfetched: crawl.unfetched,
    }
}
