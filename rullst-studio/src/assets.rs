//! Same-origin, build-time Studio assets.

use axum::{
    Router,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

const STUDIO_CSS: &str = include_str!("../assets/studio.css");
const LOGGER_JS: &str = r#"document.addEventListener("DOMContentLoaded",()=>{const target=document.getElementById("studio-request-stream");if(!target||typeof EventSource==="undefined")return;const source=new EventSource("/studio/requests/stream");source.onmessage=(event)=>{const row=document.createElement("div");row.innerHTML=event.data;while(row.lastChild)target.prepend(row.lastChild);};window.addEventListener("beforeunload",()=>source.close(),{once:true});});"#;

/// Shows the remote Studio's alert notifications: polls
/// `/studio/notifications` every 15 seconds with the cursor of this browser
/// (kept in `localStorage`, shared by its tabs) and, once the developer has
/// allowed notifications, shows each new one. The tag of a notification is
/// unique, so tabs that read the same one show it once.
const NOTIFICATIONS_JS: &str = r#"(()=>{"use strict";const KEY="rullst-studio-notifications";const supported=typeof Notification!=="undefined";const status=document.getElementById("studio-notify-status");const button=document.getElementById("studio-notify-permission");const describe=()=>{if(!status)return;if(!supported){status.textContent="This browser cannot show desktop notifications.";return;}const permission=Notification.permission;status.textContent=permission==="granted"?"Allowed in this browser.":permission==="denied"?"Blocked in this browser: allow notifications for this page in the browser's site settings.":"Not allowed in this browser yet.";};if(button&&supported){button.addEventListener("click",()=>{Notification.requestPermission().then(describe,describe);});}describe();const read=()=>{try{return JSON.parse(localStorage.getItem(KEY)||"{}")||{};}catch(_){return {};}};const write=(value)=>{try{localStorage.setItem(KEY,JSON.stringify(value));}catch(_){}};const poll=async()=>{const saved=read();const query=typeof saved.session==="string"&&Number.isSafeInteger(saved.cursor)?"?session="+encodeURIComponent(saved.session)+"&after="+saved.cursor:"";let feed;try{const response=await fetch("/studio/notifications"+query,{credentials:"same-origin",cache:"no-store"});if(!response.ok)return;feed=await response.json();}catch(_){return;}if(!feed||typeof feed.session!=="string"||!Number.isSafeInteger(feed.cursor))return;write({session:feed.session,cursor:feed.cursor});if(!feed.enabled||!supported||Notification.permission!=="granted"||!Array.isArray(feed.notifications))return;for(const item of feed.notifications){if(!item||typeof item.title!=="string"||typeof item.body!=="string")continue;const note=new Notification(item.title,{body:item.body,tag:"rullst-studio-"+feed.session+"-"+item.id});note.addEventListener("click",()=>{window.focus();window.location.assign("/studio/alerts");note.close();});}};poll();setInterval(poll,15000);})();"#;

pub(crate) fn router() -> Router {
    Router::new()
        .route("/studio.css", get(studio_css))
        .route("/logger.js", get(logger_js))
        .route("/notifications.js", get(notifications_js))
}

async fn studio_css() -> Response {
    asset_response("text/css; charset=utf-8", STUDIO_CSS)
}

async fn logger_js() -> Response {
    asset_response("application/javascript; charset=utf-8", LOGGER_JS)
}

async fn notifications_js() -> Response {
    asset_response("application/javascript; charset=utf-8", NOTIFICATIONS_JS)
}

fn asset_response(content_type: &'static str, body: &'static str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn assets_are_same_origin_typed_and_self_contained() {
        for (path, content_type) in [
            ("/studio.css", "text/css; charset=utf-8"),
            ("/logger.js", "application/javascript; charset=utf-8"),
            ("/notifications.js", "application/javascript; charset=utf-8"),
        ] {
            let response = router()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .body(Body::empty())
                        .expect("asset request"),
                )
                .await
                .expect("asset response");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers().get(header::CONTENT_TYPE),
                Some(&axum::http::HeaderValue::from_static(content_type)),
            );
        }
        assert!(!STUDIO_CSS.contains("cdn.tailwindcss.com"));
        assert!(!LOGGER_JS.contains("unpkg.com"));
        assert!(NOTIFICATIONS_JS.contains("\"/studio/notifications\""));
        assert!(!NOTIFICATIONS_JS.contains("innerHTML"));
    }
}
