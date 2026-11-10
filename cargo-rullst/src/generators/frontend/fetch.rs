//! The only network access of the frontend toolchain: one HTTPS download into
//! a caller-provided sink, bounded in size. Tests use a fixture `Fetcher`.

use std::io::Write;
use std::time::Duration;

/// Why a download failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FetchError {
    #[error("the download could not start: {0}")]
    Client(String),
    #[error("{url} answered HTTP {status}")]
    Status { url: String, status: u16 },
    #[error("the download from {url} exceeded {limit} bytes")]
    TooLarge { url: String, limit: u64 },
    #[error("the download from {url} failed: {reason}")]
    Transfer { url: String, reason: String },
    #[error("the downloaded bytes could not be written: {0}")]
    Write(std::io::ErrorKind),
}

/// Downloads a URL into a sink.
pub(crate) trait Fetcher {
    /// Writes the body of `url` to `sink` and returns its length; fails
    /// beyond `max_bytes`.
    fn fetch<W: Write + Send>(
        &self,
        url: &str,
        max_bytes: u64,
        sink: &mut W,
    ) -> Result<u64, FetchError>;
}

/// HTTPS downloads with `reqwest` (redirects allowed, HTTPS only).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HttpFetcher;

impl Fetcher for HttpFetcher {
    #[cfg_attr(mutants, mutants::skip)]
    fn fetch<W: Write + Send>(
        &self,
        url: &str,
        max_bytes: u64,
        sink: &mut W,
    ) -> Result<u64, FetchError> {
        // Its own thread and runtime: callers may already run inside Tokio
        // (`cargo rullst dev`), where a nested runtime cannot block.
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| FetchError::Client(error.to_string()))?;
                    runtime.block_on(download(url, max_bytes, sink))
                })
                .join()
                .unwrap_or_else(|_| Err(FetchError::Client("the download thread stopped".into())))
        })
    }
}

#[cfg_attr(mutants, mutants::skip)]
async fn download<W: Write>(url: &str, max_bytes: u64, sink: &mut W) -> Result<u64, FetchError> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .user_agent(concat!(
            "cargo-rullst/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/Rullst/Rullst)"
        ))
        .build()
        .map_err(|error| FetchError::Client(error.to_string()))?;
    let transfer = |error: reqwest::Error| FetchError::Transfer {
        url: url.to_owned(),
        reason: error.to_string(),
    };
    let mut response = client.get(url).send().await.map_err(transfer)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(FetchError::Status {
            url: url.to_owned(),
            status: response.status().as_u16(),
        });
    }
    let mut written: u64 = 0;
    while let Some(chunk) = response.chunk().await.map_err(transfer)? {
        written = written.saturating_add(chunk.len() as u64);
        if written > max_bytes {
            return Err(FetchError::TooLarge {
                url: url.to_owned(),
                limit: max_bytes,
            });
        }
        sink.write_all(&chunk)
            .map_err(|error| FetchError::Write(error.kind()))?;
    }
    Ok(written)
}

/// Serves fixed bytes (or a fixed error) for every URL and records the
/// requests: the tests' only "network".
#[cfg(test)]
pub(crate) struct Fixture {
    pub body: Result<Vec<u8>, FetchError>,
    pub calls: std::cell::Cell<usize>,
    pub urls: std::cell::RefCell<Vec<String>>,
}

#[cfg(test)]
impl Fixture {
    pub(crate) fn serving(body: &[u8]) -> Self {
        Self {
            body: Ok(body.to_vec()),
            calls: std::cell::Cell::new(0),
            urls: Default::default(),
        }
    }

    pub(crate) fn failing(error: FetchError) -> Self {
        Self {
            body: Err(error),
            calls: std::cell::Cell::new(0),
            urls: Default::default(),
        }
    }
}

#[cfg(test)]
impl Fetcher for Fixture {
    fn fetch<W: Write + Send>(
        &self,
        url: &str,
        max_bytes: u64,
        sink: &mut W,
    ) -> Result<u64, FetchError> {
        self.calls.set(self.calls.get() + 1);
        self.urls.borrow_mut().push(url.to_owned());
        let body = self.body.clone()?;
        if body.len() as u64 > max_bytes {
            return Err(FetchError::TooLarge {
                url: url.to_owned(),
                limit: max_bytes,
            });
        }
        sink.write_all(&body)
            .map_err(|error| FetchError::Write(error.kind()))?;
        Ok(body.len() as u64)
    }
}
