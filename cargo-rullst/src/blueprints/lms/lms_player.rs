//! CSP-compatible standalone lesson player appended to the generated LMS page module.

pub(super) const PLAYER_PAGE: &str = r##"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LessonMediaError {
    InvalidKind,
    InvalidSource,
    MissingCaptions,
    InvalidLanguage,
    InvalidTranscript,
}

fn valid_media_source(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 2_048
        || value.bytes().any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return false;
    }
    let Ok(uri) = value.parse::<rullst::server::Uri>() else {
        return false;
    };
    match uri.scheme_str() {
        Some("https") => uri.authority().is_some(),
        None => uri.authority().is_none()
            && uri.path().starts_with('/')
            && !uri.path().starts_with("//"),
        Some(_) => false,
    }
}

fn valid_language_tag(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 35
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

pub fn lesson_player_page(
    title: &str,
    media_kind: &str,
    media_url: &str,
    captions_url: &str,
    transcript: &str,
    language_tag: &str,
    course_id: i32,
    lesson_id: i32,
    progress_percent: i32,
    csrf_token: &str,
    progress_key: &str,
    csp_nonce: &str,
) -> Result<String, LessonMediaError> {
    if !valid_media_source(media_url) {
        return Err(LessonMediaError::InvalidSource);
    }
    if !valid_language_tag(language_tag) {
        return Err(LessonMediaError::InvalidLanguage);
    }
    if transcript.is_empty() || transcript.len() > 65_536 {
        return Err(LessonMediaError::InvalidTranscript);
    }
    let media_player = match media_kind {
        "video" => {
            if !valid_media_source(captions_url) {
                return Err(LessonMediaError::MissingCaptions);
            }
            html! {
                <video controls="controls" preload="metadata">
                    <source src={media_url} />
                    <track kind="captions" src={captions_url} srclang={language_tag} label={language_tag} default="true" />
                    "Your browser does not support HTML video. Use the transcript below."
                </video>
            }
        }
        "audio" => html! {
            <audio controls="controls" preload="metadata">
                <source src={media_url} />
                "Your browser does not support HTML audio. Use the transcript below."
            </audio>
        },
        _ => return Err(LessonMediaError::InvalidKind),
    };
    Ok(html! {
        <html lang="en" class="dark">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>{title}</title>
                <style nonce={csp_nonce}>
                    "
                    * { box-sizing: border-box; }
                    body { margin: 0; background: #080b11; color: #f8fafc; min-height: 100vh; padding: 2rem 1rem 4rem; font: 16px system-ui, sans-serif; }
                    a { color: #fdba74; font-weight: 700; }
                    a:focus-visible, button:focus-visible, video:focus-visible, audio:focus-visible, summary:focus-visible { outline: 3px solid #fbbf24; outline-offset: 3px; }
                    main { max-width: 60rem; margin: 0 auto; }
                    .player-card { overflow: hidden; margin-top: 1.5rem; border: 1px solid #334155; border-radius: 1rem; background: #0f172a; }
                    video { display: block; width: 100%; aspect-ratio: 16 / 9; background: #000; }
                    audio { display: block; width: calc(100% - 3rem); margin: 1.5rem; }
                    .info { padding: 1.5rem; }
                    h1 { margin-top: 0; }
                    .notice { color: #cbd5e1; line-height: 1.6; }
                    .progress { color: #6ee7b7; font-weight: 700; }
                    .progress-form { display: flex; gap: .75rem; flex-wrap: wrap; margin-top: 1.25rem; }
                    .transcript { margin-top: 1.5rem; border-top: 1px solid #334155; padding-top: 1rem; }
                    .transcript summary { cursor: pointer; font-weight: 800; }
                    .transcript p { color: #e2e8f0; line-height: 1.8; white-space: pre-wrap; }
                    button { border: 1px solid #34d399; border-radius: .55rem; background: #047857; color: #fff; padding: .7rem 1rem; font: inherit; font-weight: 800; cursor: pointer; }
                    button:hover { background: #065f46; }
                    "
                </style>
            </head>
            <body>
                <main>
                    <a href={format!("/courses/{course_id}")}>"← Back to course"</a>
                    <article class="player-card">
                        {rullst::html::RawHtml(media_player)}
                        <div class="info">
                            <h1>{title}</h1>
                            <p class="notice">"This development fixture may require an explicit media-src policy. Production applications must provide approved same-origin or signed media and verify caption/transcript quality."</p>
                            <p class="progress" role="status">"Saved progress: "{progress_percent.to_string()}"%"</p>
                            <details class="transcript">
                                <summary>"Transcript ("{language_tag}")"</summary>
                                <p>{transcript}</p>
                            </details>
                            <form class="progress-form" method="post" action={format!("/lessons/{lesson_id}/progress")}>
                                <input type="hidden" name="_token" value={csrf_token} />
                                <input type="hidden" name="idempotency_key" value={progress_key} />
                                <button type="submit" name="progress_percent" value="25">"Save 25%"</button>
                                <button type="submit" name="progress_percent" value="50">"Save 50%"</button>
                                <button type="submit" name="progress_percent" value="100">"Mark complete"</button>
                            </form>
                        </div>
                    </article>
                </main>
            </body>
        </html>
    })
}
"##;

#[cfg(test)]
mod tests {
    use super::PLAYER_PAGE;

    #[test]
    fn player_is_accessible_bounded_nonce_based_and_does_not_autoplay() {
        assert!(PLAYER_PAGE.contains("<style nonce={csp_nonce}>"));
        assert!(PLAYER_PAGE.contains("<track kind=\"captions\""));
        assert!(PLAYER_PAGE.contains("<audio controls=\"controls\""));
        assert!(PLAYER_PAGE.contains("InvalidTranscript"));
        assert!(PLAYER_PAGE.contains("value.len() > 2_048"));
        assert!(!PLAYER_PAGE.contains("autoplay"));
        assert!(!PLAYER_PAGE.contains("style="));
        assert!(!PLAYER_PAGE.contains("hx-"));
    }
}
