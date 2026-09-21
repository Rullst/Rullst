//! Controlled provider/browser fixture, not evidence of Bunny playback conformance.
use super::*;
use axum::{
    body::Bytes,
    extract::Request,
    middleware::Next,
    routing::{any, head},
};
use subtle::ConstantTimeEq;

pub struct Upload {
    pub video: String,
    pub length: usize,
    pub bytes: Vec<u8>,
}

pub fn routes(router: Router<Arc<Mutex<Remote>>>) -> Router<Arc<Mutex<Remote>>> {
    router
        .route("/tusupload", any(create_upload))
        .route(
            "/tusupload/{id}",
            head(upload_head).patch(upload_patch).options(options),
        )
        .route("/embed/{library}/{video}", get(embed))
        .route("/media/{video}", get(media))
        .route(
            "/captions.vtt",
            get(|| async {
                (
                    [("content-type", "text/vtt")],
                    "WEBVTT\n\n00:00.000 --> 00:01.000\nRust lesson.\n",
                )
            }),
        )
        .layer(axum::extract::DefaultBodyLimit::max(2_097_152))
        .layer(axum::middleware::from_fn(cors))
}
async fn cors(request: Request, next: Next) -> Response {
    let origin = request.headers().get("origin").cloned();
    let mut response = next.run(request).await;
    if let Some(origin) = origin {
        let allowed = origin.to_str().ok().is_some_and(|s| {
            s.strip_prefix("http://127.0.0.1:")
                .is_some_and(|p| p.parse::<u16>().is_ok())
        });
        if allowed {
            response
                .headers_mut()
                .insert("access-control-allow-origin", origin);
            response.headers_mut().insert(
                "access-control-allow-methods",
                "POST,HEAD,PATCH,OPTIONS".parse().unwrap(),
            );
            response.headers_mut().insert("access-control-allow-headers","Content-Type,Tus-Resumable,AuthorizationSignature,AuthorizationExpire,LibraryId,VideoId,Upload-Length,Upload-Metadata,Upload-Offset,Cache-Control".parse().unwrap());
            response.headers_mut().insert(
                "access-control-expose-headers",
                "Location,Upload-Offset,Upload-Length,Tus-Resumable"
                    .parse()
                    .unwrap(),
            );
        }
    }
    response
}
async fn options() -> Response {
    response(204, [])
}
fn response<const N: usize>(status: u16, headers: [(&'static str, String); N]) -> Response {
    let mut response = StatusCode::from_u16(status).unwrap().into_response();
    response
        .headers_mut()
        .insert("tus-resumable", "1.0.0".parse().unwrap());
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    for (name, value) in headers {
        response.headers_mut().insert(name, value.parse().unwrap());
    }
    response
}
fn value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}
fn sha(value: &str) -> String {
    ring::digest::digest(&ring::digest::SHA256, value.as_bytes())
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn authorize(headers: &HeaderMap) -> Option<String> {
    if value(headers, "Tus-Resumable") != Some("1.0.0")
        || value(headers, "LibraryId") != Some("7")
        || headers.contains_key("cookie")
    {
        return None;
    }
    let video = VideoId::new(value(headers, "VideoId")?).ok()?;
    let expires = value(headers, "AuthorizationExpire")?.parse::<i64>().ok()?;
    if expires <= SystemClock.now().ok()? {
        return None;
    }
    let expected = sha(&format!("7{API}{expires}{}", video.as_str()));
    if expected
        .as_bytes()
        .ct_eq(value(headers, "AuthorizationSignature")?.as_bytes())
        .unwrap_u8()
        != 1
    {
        return None;
    }
    Some(video.as_str().to_owned())
}
async fn create_upload(State(state): State<Arc<Mutex<Remote>>>, request: Request) -> Response {
    if request.method() == axum::http::Method::OPTIONS {
        return options().await;
    }
    if request.method() != axum::http::Method::POST {
        return response(405, []);
    }
    let Some(video) = authorize(request.headers()) else {
        return response(401, []);
    };
    let length = value(request.headers(), "Upload-Length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    if length == 0 || length > 2_097_152 {
        return response(413, []);
    }
    let mut state = state.lock().unwrap();
    if !state.videos.contains_key(&video) {
        return response(404, []);
    }
    let id = format!("upload-{}", state.uploads.len() + 1);
    state.uploads.insert(
        id.clone(),
        Upload {
            video,
            length,
            bytes: Vec::new(),
        },
    );
    state.calls.push("tus-post".into());
    response(201, [("location", format!("/tusupload/{id}"))])
}
async fn upload_head(
    State(state): State<Arc<Mutex<Remote>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(video) = authorize(&headers) else {
        return response(401, []);
    };
    let mut state = state.lock().unwrap();
    state.calls.push("tus-head".into());
    let Some(upload) = state.uploads.get(&id).filter(|u| u.video == video) else {
        return response(404, []);
    };
    response(
        200,
        [
            ("upload-offset", upload.bytes.len().to_string()),
            ("upload-length", upload.length.to_string()),
        ],
    )
}
async fn upload_patch(
    State(state): State<Arc<Mutex<Remote>>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Response {
    let Some(video) = authorize(&headers) else {
        return response(401, []);
    };
    let offset = value(&headers, "Upload-Offset").and_then(|v| v.parse::<usize>().ok());
    let mut state = state.lock().unwrap();
    state.calls.push("tus-patch".into());
    let Some(upload) = state.uploads.get_mut(&id).filter(|u| u.video == video) else {
        return response(404, []);
    };
    if offset != Some(upload.bytes.len()) || bytes.len() > upload.length - upload.bytes.len() {
        return response(409, []);
    }
    upload.bytes.extend_from_slice(&bytes);
    let offset = upload.bytes.len();
    let complete = offset == upload.length;
    if complete {
        state.videos.get_mut(&video).unwrap()["status"] = json!(4);
    }
    response(204, [("upload-offset", offset.to_string())])
}
fn playback(video: &str, query: &BTreeMap<String, String>) -> bool {
    let Some(expiry) = query.get("expires").and_then(|v| v.parse::<i64>().ok()) else {
        return false;
    };
    let Some(token) = query.get("token") else {
        return false;
    };
    if VideoId::new(video).is_err() || expiry <= SystemClock.now().unwrap() {
        return false;
    }
    let expected = sha(&format!("{EMBED}{video}{expiry}"));
    expected.as_bytes().ct_eq(token.as_bytes()).unwrap_u8() == 1
}
async fn embed(
    Path((library, video)): Path<(i64, String)>,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if library != 7 || !playback(&video, &query) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let url = format!(
        "/media/{video}?token={}&expires={}",
        query["token"], query["expires"]
    );
    let url = rullst_core::html::escape_str(&url);
    axum::response::Html(format!("<!doctype html><html lang=\"en\"><title>Controlled video fixture</title><video id=\"lesson\" controls=\"true\" preload=\"metadata\" width=\"320\" src=\"{url}\"><track kind=\"captions\" src=\"/captions.vtt\" srclang=\"en\" label=\"English\" default=\"true\" /></video></html>")).into_response()
}
async fn media(
    State(state): State<Arc<Mutex<Remote>>>,
    Path(video): Path<String>,
    Query(query): Query<BTreeMap<String, String>>,
) -> Response {
    if !playback(&video, &query) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let state = state.lock().unwrap();
    if !state.videos.contains_key(&video) {
        return StatusCode::NOT_FOUND.into_response();
    }
    (
        [
            ("content-type", "video/webm"),
            ("cache-control", "private, no-store"),
        ],
        include_bytes!("../fixtures/lesson.webm").as_slice(),
    )
        .into_response()
}
