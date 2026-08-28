//! CSP-compatible standalone lesson player appended to the generated LMS page module.

pub(super) const PLAYER_PAGE: &str = r##"
pub fn video_player_page(
    title: &str,
    video_url: &str,
    course_id: i32,
    lesson_id: i32,
    progress_percent: i32,
    csrf_token: &str,
    progress_key: &str,
    csp_nonce: &str,
) -> String {
    html! {
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
                    a:focus-visible, button:focus-visible, video:focus-visible { outline: 3px solid #fbbf24; outline-offset: 3px; }
                    main { max-width: 60rem; margin: 0 auto; }
                    .player-card { overflow: hidden; margin-top: 1.5rem; border: 1px solid #334155; border-radius: 1rem; background: #0f172a; }
                    video { display: block; width: 100%; aspect-ratio: 16 / 9; background: #000; }
                    .info { padding: 1.5rem; }
                    h1 { margin-top: 0; }
                    .notice { color: #cbd5e1; line-height: 1.6; }
                    .progress { color: #6ee7b7; font-weight: 700; }
                    .progress-form { display: flex; gap: .75rem; flex-wrap: wrap; margin-top: 1.25rem; }
                    button { border: 1px solid #34d399; border-radius: .55rem; background: #047857; color: #fff; padding: .7rem 1rem; font: inherit; font-weight: 800; cursor: pointer; }
                    button:hover { background: #065f46; }
                    "
                </style>
            </head>
            <body>
                <main>
                    <a href={format!("/courses/{course_id}")}>"← Back to course"</a>
                    <article class="player-card">
                        <video controls="controls" preload="metadata" src={video_url}>
                            "Your browser does not support HTML video."
                        </video>
                        <div class="info">
                            <h1>{title}</h1>
                            <p class="notice">"This development fixture may require an explicit media-src policy. Production applications must provide approved same-origin or signed media, captions and a transcript."</p>
                            <p class="progress" role="status">"Saved progress: "{progress_percent.to_string()}"%"</p>
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
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::PLAYER_PAGE;

    #[test]
    fn player_is_standalone_nonce_based_and_does_not_autoplay() {
        assert!(PLAYER_PAGE.contains("<style nonce={csp_nonce}>"));
        assert!(PLAYER_PAGE.contains("captions and a transcript"));
        assert!(!PLAYER_PAGE.contains("autoplay"));
        assert!(!PLAYER_PAGE.contains("style="));
        assert!(!PLAYER_PAGE.contains("hx-"));
    }
}
