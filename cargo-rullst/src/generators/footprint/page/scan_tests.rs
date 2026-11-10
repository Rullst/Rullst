use super::*;

fn urls(references: &[Reference]) -> Vec<(&str, Kind)> {
    references
        .iter()
        .map(|reference| (reference.url.as_str(), reference.kind))
        .collect()
}

#[test]
fn a_page_lists_the_resources_a_browser_loads() {
    let page = r#"<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<title>Home <img src="/in-title.png"></title>
<link rel="stylesheet" href="/static/app.css?v=2">
<link rel="alternate stylesheet" href="/static/print.css">
<LINK REL="Preload" AS="font" HREF='/static/inter.woff2' crossorigin>
<link rel=modulepreload href=/static/module.js>
<link rel="preload" as="image" href="/static/hero.avif">
<link rel="preload" as="fetch" href="/api/data">
<link rel="icon" href="/favicon.svg">
<link rel="manifest" href="/manifest.webmanifest">
<link rel="preconnect" href="https://fonts.example">
<link rel="dns-prefetch" href="//cdn.example">
<link rel="prefetch" href="/next.html">
<link rel="canonical" href="https://example.com/">
<script src="/static/htmx.min.js" nonce="abc" defer></script>
<script nonce="abc">document.write('<img src="/from-script.png">')</script>
<style nonce="abc">body { background: url("/static/bg.png") }</style>
</head><body>
<!-- <img src="/commented.png"> -->
<img src="/static/logo.png" alt="logo">
<img srcset="/static/a-1x.webp 1x, /static/a-2x.webp 2x" alt="">
<img src="" srcset="/static/b.webp 480w">
<img alt="no source">
<video poster="/static/poster.jpg"><source src="/static/movie.mp4"></video>
<audio src="/static/sound.mp3"></audio>
<iframe src="https://player.example/embed"></iframe>
<div style="background-image:url(/static/tile.png)"></div>
<template><img src="/template.png"></template>
<noscript><img src="/noscript.png"></noscript>
<textarea><img src="/textarea.png"></textarea>
<a href="/about">About</a>
<script src="https://analytics.example/a.js"></script>
</body></html>"#;
    let scan = html(page);
    assert_eq!(scan.base, None);
    assert_eq!(
        urls(&scan.references),
        [
            ("/static/app.css?v=2", Kind::Stylesheet),
            ("/static/inter.woff2", Kind::Font),
            ("/static/module.js", Kind::Script),
            ("/static/hero.avif", Kind::Image),
            ("/api/data", Kind::Other),
            ("/favicon.svg", Kind::Image),
            ("/manifest.webmanifest", Kind::Manifest),
            ("https://fonts.example", Kind::Connection),
            ("//cdn.example", Kind::Connection),
            ("/static/htmx.min.js", Kind::Script),
            ("/static/bg.png", Kind::Image),
            ("/static/logo.png", Kind::Image),
            ("/static/a-1x.webp", Kind::Image),
            ("/static/b.webp", Kind::Image),
            ("/static/poster.jpg", Kind::Image),
            ("https://player.example/embed", Kind::Document),
            ("/static/tile.png", Kind::Image),
            ("https://analytics.example/a.js", Kind::Script),
        ]
    );
}

#[test]
fn the_base_entities_and_malformed_markup_are_handled() {
    let scan = html(
        "<base href=\" /assets/ \"><base href=\"/other/\"><img src=\"a.png?x=1&amp;y=&#50;&#x33;&bogus;&\">",
    );
    assert_eq!(scan.base.as_deref(), Some("/assets/"));
    assert_eq!(
        urls(&scan.references),
        [("a.png?x=1&y=23&bogus;&", Kind::Image)]
    );
    assert_eq!(html("<base href=\"\">").base, None);

    // Unterminated markup ends the scan without a panic.
    for broken in [
        "<img src=\"/a.png",
        "<img src=/a.png",
        "<img",
        "<",
        "<!--",
        "< img src=/x.png>",
        "</img src=/x.png>",
        "<script src=/s.js>never closed",
        "<img src=\u{e9}\u{1f600}>",
        "<link rel=stylesheet href>",
        "<img =\"/x\" src='/y.png'>",
    ] {
        let _ = html(broken);
    }
    assert_eq!(
        urls(&html("<img src=\"/a.png").references),
        [("/a.png", Kind::Image)]
    );
    assert_eq!(
        urls(&html("<img src=/a.png").references),
        [("/a.png", Kind::Image)]
    );
    assert!(html("< img src=/x.png>").references.is_empty());
    assert!(html("<link rel=stylesheet href>").references.is_empty());
    assert_eq!(
        urls(&html("<script src=/s.js>never closed<img src=/x.png>").references),
        [("/s.js", Kind::Script)]
    );
    assert_eq!(
        urls(&html("<img =\"/x\" src='/y.png'>").references),
        [("/y.png", Kind::Image)]
    );
    assert_eq!(
        urls(&html("<img src=\u{e9}\u{1f600}>").references),
        [("\u{e9}\u{1f600}", Kind::Image)]
    );
    assert_eq!(
        urls(&html("<IMG\nSRC = \"/spaced.png\" / >").references),
        [("/spaced.png", Kind::Image)]
    );
    assert_eq!(decode("a&b"), "a&b");
    assert_eq!(decode("&lt;&gt;&quot;&apos;"), "<>\"'");
    assert_eq!(
        decode("&#xZZ;&verylongentityname;"),
        "&#xZZ;&verylongentityname;"
    );
    assert_eq!(decode("&#X41;"), "A");
}

#[test]
fn stylesheets_list_imports_fonts_and_images() {
    let sheet = r#"
@import "base.css";
@import url('theme.css') screen;
@import url(print.css) print;
/* url(/commented.png) @import "commented.css"; */
.logo { background: url(/img/logo.svg) no-repeat, url( "/img/shade.png" ); }
.quote::before { content: "url(not-a-url.png)"; }
.mask { mask: url(#clip); }
@font-face {
  font-family: Inter;
  src: url("/fonts/inter.woff2") format("woff2"), url("/fonts/inter.woff") format("woff");
}
@font-face { font-family: X; SRC: url(/fonts/x.ttf) }
.icon { background: url(/img/icon.png?v=1#frag); cursor: url(/img/pointer.cur), auto; }
"#;
    assert_eq!(
        urls(&css(sheet)),
        [
            ("base.css", Kind::Stylesheet),
            ("theme.css", Kind::Stylesheet),
            ("print.css", Kind::Stylesheet),
            ("/img/logo.svg", Kind::Image),
            ("/img/shade.png", Kind::Image),
            ("/fonts/inter.woff2", Kind::Font),
            ("/fonts/x.ttf", Kind::Font),
            ("/img/icon.png?v=1#frag", Kind::Image),
            ("/img/pointer.cur", Kind::Other),
        ]
    );
    for (url, kind) in [
        ("a.WOFF", Kind::Font),
        ("a.woff", Kind::Font),
        ("a.otf", Kind::Font),
        ("a.eot", Kind::Font),
        ("a.css?x", Kind::Stylesheet),
        ("a.jpeg", Kind::Image),
        ("a.jpg", Kind::Image),
        ("a.gif", Kind::Image),
        ("a.webp", Kind::Image),
        ("a.avif", Kind::Image),
        ("a.ico", Kind::Image),
        ("a.bmp", Kind::Image),
        ("a.png", Kind::Image),
        ("noextension", Kind::Other),
    ] {
        assert_eq!(css_kind(url), kind, "{url}");
    }
    for broken in [
        "url(",
        "url(\"x",
        "@import",
        "@import \"x",
        "@import x",
        "/* open",
        "a { content: \"open",
    ] {
        let _ = css(broken);
    }
    assert_eq!(urls(&css("url(\"/x.png")), [("/x.png", Kind::Image)]);
    assert!(css("@import x").is_empty());
    assert!(css("url()").is_empty());
    assert_eq!(
        urls(&css("a{src:url(/one.woff)} b{src:url(/two.woff)}")),
        [("/one.woff", Kind::Font), ("/two.woff", Kind::Font)]
    );
}

#[test]
fn kinds_have_stable_names() {
    let names: Vec<&str> = [
        Kind::Document,
        Kind::Stylesheet,
        Kind::Script,
        Kind::Font,
        Kind::Image,
        Kind::Manifest,
        Kind::Other,
        Kind::Connection,
    ]
    .into_iter()
    .map(Kind::as_str)
    .collect();
    assert_eq!(
        names,
        [
            "document",
            "stylesheet",
            "script",
            "font",
            "image",
            "manifest",
            "other",
            "connection"
        ]
    );
    assert_eq!(first_candidate(" /a.png 1x, /b.png 2x"), Some("/a.png"));
    assert_eq!(first_candidate(""), None);
}
