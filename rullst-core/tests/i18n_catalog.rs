//! A compile-time catalog end to end: `t!` in `html!`, plural rules,
//! `<html lang>` from the negotiated locale and the CSRF-protected switch.
extern crate rullst_core as rullst;

mod i18n {
    rullst::i18n::catalog!(default = "en", dir = "tests/i18n/locales");
}

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use i18n::{LOCALES, t};
use rullst::config::Environment;
use rullst::html;
use rullst::html::Markup;
use rullst::i18n::Locale;
use std::borrow::Cow;
use tower::ServiceExt;

fn locale(tag: &str) -> Locale {
    LOCALES.get(tag).expect("a catalog locale")
}

#[test]
fn the_catalog_lists_its_locales_and_default() {
    let tags: Vec<&str> = LOCALES.iter().map(|l| l.tag()).collect();
    assert_eq!(tags, ["en", "es", "pt-BR"]);
    assert_eq!(LOCALES.default_locale().tag(), "en");
}

#[test]
fn messages_follow_the_locale() {
    let title: Cow<'static, str> = t!(locale("pt-BR"), "dashboard.title");
    assert_eq!(title, "Painel");
    assert!(matches!(title, Cow::Borrowed(_)));
    assert_eq!(t!(locale("es"), "dashboard.title"), "Panel");
    let en = locale("en");
    assert_eq!(t!(&en, "dashboard.greeting", name = "Ana"), "Hello, Ana!");
    struct Ctx {
        locale: Locale,
    }
    let ctx = Ctx {
        locale: locale("es"),
    };
    let name = String::from("Bea");
    assert_eq!(
        t!(ctx.locale, "dashboard.greeting", name = name),
        "¡Hola, Bea!"
    );
    assert_eq!(
        t!(en, "dashboard.compare", b = 2, a = 1),
        "1 < 2 & {braces}"
    );
}

#[test]
fn plural_forms_use_each_language_rule() {
    let unread = |tag, count: i64| t!(locale(tag), "inbox.unread", count = count).into_owned();
    assert_eq!(unread("en", 1), "You have one unread message");
    assert_eq!(unread("en", -1), "You have one unread message");
    assert_eq!(unread("en", 0), "You have 0 unread messages");
    assert_eq!(unread("es", 1), "Tienes un mensaje sin leer");
    assert_eq!(unread("es", 0), "Tienes 0 mensajes sin leer");
    assert_eq!(unread("pt-BR", 0), "Você tem 0 mensagem não lida");
    assert_eq!(unread("pt-BR", 1), "Você tem 1 mensagem não lida");
    assert_eq!(unread("pt-BR", 2), "Você tem 2 mensagens não lidas");
    let items = [(); 3];
    assert_eq!(
        t!(locale("en"), "inbox.unread", count = items.len()),
        "You have 3 unread messages"
    );
}

#[test]
fn html_escapes_messages_and_arguments() {
    let en = locale("en");
    let page = html! {
        <p title={t!(en, "dashboard.greeting", name = "\"x\"")}>
            {t!(en, "dashboard.greeting", name = "<script>alert(1)</script>")}
        </p>
        <span>{t!(en, "dashboard.compare", a = "<i>", b = 2)}</span>
    };
    assert_eq!(
        page,
        "<p title=\"Hello, &quot;x&quot;!\">Hello, &lt;script&gt;alert(1)&lt;/script&gt;!</p>\
         <span>&lt;i&gt; &lt; 2 &amp; {braces}</span>"
    );
}

#[test]
fn html_keys_are_trusted_markup_with_escaped_arguments() {
    let terms: Markup = t!(locale("pt-BR"), "legal.terms_html", name = "<b>Ana</b>");
    let page = html! { <label>{terms}</label> };
    assert_eq!(
        page,
        "<label>Aceito os <a href=\"/terms\">termos</a>, &lt;b&gt;Ana&lt;/b&gt;</label>"
    );
}

async fn body(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), 1 << 16).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn the_negotiated_locale_sets_html_lang() {
    async fn dashboard(locale: Locale, htmx: rullst::HtmxRequest) -> axum::response::Html<String> {
        let content = html! { <h1>{t!(locale, "dashboard.greeting", name = "Ana")}</h1> };
        rullst::render_page_with_lang(&htmx, locale.tag(), &t!(locale, "dashboard.title"), content)
    }
    let app = rullst::Router::new()
        .route("/", axum::routing::get(dashboard))
        .layer(LOCALES.layer())
        .into_axum();
    let request = Request::get("/")
        .header(header::ACCEPT_LANGUAGE, "pt-PT, en;q=0.5")
        .body(Body::empty())
        .unwrap();
    let page = body(app.oneshot(request).await.unwrap()).await;
    assert!(page.contains("<html lang=\"pt-BR\""), "{page}");
    assert!(page.contains("<title>Painel</title>"), "{page}");
    assert!(page.contains("<h1>Olá, Ana!</h1>"), "{page}");
}

/// The language menu from the guide (`docs/src/i18n.md`).
async fn menu(
    locale: Locale,
    axum::Extension(csrf): axum::Extension<rullst::security::CsrfToken>,
) -> Markup {
    html! {
        <form method="post" action="/locale">
            <input type="hidden" name="_token" value={csrf.as_str()} />
            <input type="hidden" name="next" value="/" />
            <label>{t!(locale, "nav.language")}
                <select name="locale">
                    @for option in LOCALES.iter() {
                        <option value={option.tag()} selected?={option == locale}>{option.tag()}</option>
                    }
                </select>
            </label>
            <button>"OK"</button>
        </form>
    }
}

#[tokio::test]
async fn the_language_menu_marks_the_current_locale() {
    let app = rullst::Router::new()
        .route("/menu", axum::routing::get(menu))
        .layer(LOCALES.layer())
        .into_axum()
        .layer(axum::middleware::from_fn(rullst::security::csrf_middleware))
        .layer(axum::Extension(Environment::Development));
    let request = Request::get("/menu")
        .header(header::COOKIE, "rullst_csrf=token123; rullst_locale=es")
        .body(Body::empty())
        .unwrap();
    let page = body(app.oneshot(request).await.unwrap()).await;
    assert!(page.contains("value=\"token123\""), "{page}");
    assert!(
        page.contains("<label>Idioma<select name=\"locale\">"),
        "{page}"
    );
    assert!(
        page.contains(
            "<option value=\"en\">en</option><option value=\"es\" selected>es</option>\
             <option value=\"pt-BR\">pt-BR</option>"
        ),
        "{page}"
    );
}

fn switch_app() -> axum::Router {
    rullst::Router::new()
        .route("/locale", LOCALES.switch_route())
        .layer(LOCALES.layer())
        .into_axum()
        .layer(axum::middleware::from_fn(rullst::security::csrf_middleware))
        .layer(axum::Extension(Environment::Development))
}

fn switch(cookie: Option<&str>, form: &str) -> Request<Body> {
    let mut request =
        Request::post("/locale").header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    request.body(Body::from(form.to_owned())).unwrap()
}

fn locale_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("rullst_locale="))
}

#[tokio::test]
async fn the_switch_route_needs_the_csrf_token() {
    let response = switch_app()
        .oneshot(switch(None, "locale=pt-BR&next=/"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(locale_cookie(response.headers()), None);
    assert_eq!(body(response).await, "CSRF token cookie missing");

    let wrong = switch(Some("rullst_csrf=token123"), "_token=other456&locale=pt-BR");
    let response = switch_app().oneshot(wrong).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(locale_cookie(response.headers()), None);

    let valid = switch(
        Some("rullst_csrf=token123"),
        "_token=token123&locale=pt-BR&next=%2Finbox",
    );
    let response = switch_app().oneshot(valid).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[header::LOCATION], "/inbox");
    assert_eq!(
        locale_cookie(response.headers()),
        Some("rullst_locale=pt-BR; Path=/; Max-Age=31536000; SameSite=Lax; HttpOnly")
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, "rullst_locale=pt-BR".parse().unwrap());
    headers.insert(header::ACCEPT_LANGUAGE, "es".parse().unwrap());
    assert_eq!(LOCALES.negotiate(&headers).tag(), "pt-BR");
}
