use super::*;

const STRICT: &str =
    "default-src 'self'; script-src 'self' 'nonce-n0nce'; style-src 'self' 'nonce-n0nce'";

fn kinds(html: &str, policy: Option<&str>) -> Vec<HtmlIssueKind> {
    html_issues(html, policy)
        .iter()
        .map(HtmlIssue::kind)
        .collect()
}

fn clean(html: &str) {
    assert_eq!(html_issues(html, Some(STRICT)), [], "{html}");
}

fn only(html: &str, kind: HtmlIssueKind) {
    assert_eq!(kinds(html, Some(STRICT)), [kind], "{html}");
}

#[test]
fn a_well_formed_rullst_page_has_no_issues() {
    clean(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\" />\
         <title>Caf\u{e9} &amp; co &lt; 3</title>\
         <script nonce=\"n0nce\" src=\"/static/htmx-1.9.12.min.js\"></script>\
         <meta name=\"htmx-config\" content=\"{&quot;includeIndicatorStyles&quot;:false}\" />\
         <style nonce=\"n0nce\">.a>.b{opacity:0}</style>\
         <script type=\"application/json\">{\"a\": \"<b>\"}</script>\
         <!-- a comment with <b> and \"quotes --></head>\
         <body><main class=\"x\"><input type=\"checkbox\" disabled checked aria-checked=\"false\" />\
         <a href=/plain title='it\"s'>Link</a><br/><img src=\"x.png\" alt=\"\">\
         <p>2 &lt; 3 and a < b, &amp;copy; &amp;#; \u{1f980}</p>\
         <pre>&lt;div&gt;</pre><code>&lt;span&gt;</code><kbd>&lt;b&gt;</kbd>\
         <samp>&lt;i&gt;</samp><textarea name=\"t\">&lt;div&gt;</textarea>\
         <script nonce=\"n0nce\">if (a < b) { x = \"&lt;div\"; }</script>\
         <script src=\"/app.js\"></script><script></script><style></style>\
         <![CDATA[ <svg> ]]></main></body></html>",
    );
}

#[test]
fn escaped_tags_and_comments_in_text_are_reported() {
    for page in [
        "<p>&lt;div&gt;</p>",
        "<p>&lt;/span&gt;</p>",
        "<p>&lt;!-- x --&gt;</p>",
        "<p>a&lt;a href</p>",
        "<p>&lt;DIV&gt;</p>",
        "<p>&lt;br/&gt;</p>",
        "<p>&lt;h2 id</p>",
        "<p>&lt;/i\n</p>",
        "<p>&lt;i</p>",
        "<p>&amp;lt;</p>",
        "<p>&amp;gt;</p>",
        "<p>&amp;amp;</p>",
        "<p>&amp;quot;</p>",
        "<p>&amp;apos;</p>",
        "<p>&amp;#39;</p>",
        "<p>&amp;#x27;</p>",
        "<p>&amp;#X2f;</p>",
        "<title>&lt;b&gt;</title>",
    ] {
        only(page, HtmlIssueKind::DoubleEscaped);
    }
    for page in [
        "<p>&lt; 3</p>",
        "<p>&lt;3</p>",
        "<p>&lt;/3</p>",
        "<p>&lt;/</p>",
        "<p>&lt;!x</p>",
        "<p>&lt;Rullst &amp; co&gt;</p>",
        "<p>&lt;bold&gt; &lt;ix &lt;/x&gt; &lt;a-b</p>",
        "<p>end &lt;</p>",
        "<p>&amp;copy;</p>",
        "<p>&amp;#;</p>",
        "<p>&amp;#x;</p>",
        "<p>&amp;#12</p>",
        "<p>&amp;#12a;</p>",
        "<p>&amp; then</p>",
    ] {
        clean(page);
    }
}

#[test]
fn the_first_escaped_form_in_a_text_run_is_the_excerpt() {
    let issues = html_issues("<p>a &amp;lt; b &lt;i</p>", None);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].excerpt(), "&amp;lt; b &lt;i</p>");
    let issues = html_issues("<p>a &lt;i b &amp;lt;</p>", None);
    assert_eq!(issues[0].excerpt(), "&lt;i b &amp;lt;</p>");
}

#[test]
fn verbatim_elements_nest_and_end() {
    clean("<pre><code>&lt;b&gt;</code> &lt;i&gt;</pre>");
    only("<code>x</code>&lt;b&gt;", HtmlIssueKind::DoubleEscaped);
    only("<code/>&lt;b&gt;", HtmlIssueKind::DoubleEscaped);
    // A stray end tag does not hide later text.
    only(
        "</code></code><code></code>&lt;b&gt;",
        HtmlIssueKind::DoubleEscaped,
    );
    clean("<CODE>&lt;b&gt;</Code>");
}

#[test]
fn raw_text_content_is_not_markup() {
    clean("<script nonce=\"n0nce\">document.write(\"<p title='>\")</script>");
    clean("<style nonce=\"n0nce\"><p>&lt;b</style>");
    clean("<textarea><p title=\"&lt;b\"></textarea>");
    clean("<SCRIPT nonce=\"n0nce\">a</Script >");
    clean("<script nonce=\"n0nce\">a</scripts></script>");
    only("<script nonce=\"n0nce\">a", HtmlIssueKind::MalformedMarkup);
    only("<title>open", HtmlIssueKind::MalformedMarkup);
}

#[test]
fn attribute_values_are_checked_for_escaped_entities_only() {
    only("<a title=\"&amp;lt;b\">x</a>", HtmlIssueKind::DoubleEscaped);
    only(
        "<a href=\"/?a=1&amp;amp;b=2\">x</a>",
        HtmlIssueKind::DoubleEscaped,
    );
    // HTML in an attribute is escaped once by design.
    clean("<a title=\"&lt;b&gt;\" href=\"/?a=1&amp;b=2\">x</a>");
}

#[test]
fn quoting_mistakes_are_malformed() {
    for page in [
        "<p title=\"Say \"hi\"\">x</p>",
        "<p title=\"a\"class=\"b\">x</p>",
        "<p title='a'b>x</p>",
        "<p a\"b=\"c\">x</p>",
        "<p a'b>x</p>",
        "<p <b>x</p>",
        "<p =\"x\">x</p>",
        "<p a=b\"c>x</p>",
        "<p a=b'c>x</p>",
        "<p a=b<c>x</p>",
        "<p a=b=c>x</p>",
        "<p a=b`c>x</p>",
        "<p a=>x</p>",
        "<p a=",
        "<p a",
        "<p title=\"open>x</p>",
        "<p",
        "<!-- open",
        "<![CDATA[ open",
        "<!DOCTYPE html",
        "<?xml open",
    ] {
        only(page, HtmlIssueKind::MalformedMarkup);
    }
    for page in [
        "<p a = \"x\" b='y' c=z d>x</p>",
        "<p title=\"x\"/>",
        "<p title=\"x\" />",
        "<p\ttitle=\"x\"\n>x</p>",
        "<br/>",
        "<p data-x=\"\u{e9}\" \u{e9}=\"\u{e9}\">\u{e9}</p>",
        "a < b <3",
        "</>",
        "< p>",
    ] {
        clean(page);
    }
}

#[test]
fn one_broken_tag_is_one_issue_at_its_first_bad_attribute() {
    let issues = html_issues("<p id=\"x\" title=\"Say \"hi\"\" class=\"a\"b>", None);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].kind(), HtmlIssueKind::MalformedMarkup);
    assert!(issues[0].excerpt().starts_with("title=\"Say"), "{issues:?}");
}

#[test]
fn boolean_attributes_set_to_false_are_reported() {
    for name in BOOLEAN_ATTRIBUTES {
        only(
            &format!("<input {name}=\"false\">"),
            HtmlIssueKind::FalseBooleanAttribute,
        );
    }
    only(
        "<button DISABLED=' False '>x</button>",
        HtmlIssueKind::FalseBooleanAttribute,
    );
    only(
        "<button disabled=false>x</button>",
        HtmlIssueKind::FalseBooleanAttribute,
    );
    clean("<button disabled=\"true\" aria-disabled=\"false\" draggable=\"false\">x</button>");
    clean("<button disabled=\"\" hidden>x</button>");
}

#[test]
fn inline_scripts_and_styles_need_the_policy_nonce() {
    for page in [
        "<script>run()</script>",
        "<script nonce=\"other\">run()</script>",
        "<script nonce>run()</script>",
        "<script type=\"module\">run()</script>",
        "<script type=\"\">run()</script>",
        "<script type=\" Text/JavaScript; charset=utf-8\">run()</script>",
        "<script type=\"application/ecmascript\">run()</script>",
        "<script type=\"text/jscript\">run()</script>",
        "<script type=\"text/livescript\">run()</script>",
        "<script type=\"importmap\">{}</script>",
        "<script type=\"speculationrules\">{}</script>",
        "<style>p{}</style>",
        "<style nonce=\"N0NCE\">p{}</style>",
    ] {
        only(page, HtmlIssueKind::MissingNonce);
    }
    for page in [
        "<script nonce=\"n0nce\">run()</script>",
        "<style nonce=\"n0nce\">p{}</style>",
        "<script src=\"/a.js\"></script>",
        "<script src>run()</script>",
        "<script></script>",
        "<style></style>",
        "<script type=\"application/ld+json\">{}</script>",
        "<script type=\"text/x-template\"><p></p></script>",
    ] {
        clean(page);
    }
    assert_eq!(kinds("<script>run()</script><style>p{}</style>", None), []);
    assert_eq!(
        kinds("<script>run()</script>", Some("script-src 'unsafe-inline'")),
        []
    );
}

#[test]
fn style_and_event_handler_attributes_need_a_permissive_policy() {
    for page in [
        "<p style=\"color:red\">x</p>",
        "<p STYLE=\"\">x</p>",
        "<button onclick=\"go()\">x</button>",
        "<button onClick=\"go()\">x</button>",
    ] {
        only(page, HtmlIssueKind::BlockedInlineAttribute);
    }
    for page in [
        "<p on=\"x\" on-click=\"x\" onclick2=\"x\" hx-on-click=\"go()\">x</p>",
        "<p data-style=\"x\">x</p>",
    ] {
        clean(page);
    }
    let open = "default-src 'self' 'unsafe-inline'";
    assert_eq!(kinds("<p style=\"x\" onclick=\"y\">x</p>", Some(open)), []);
    assert_eq!(kinds("<p style=\"x\" onclick=\"y\">x</p>", None), []);
    assert_eq!(
        kinds(
            "<p style=\"x\">x</p>",
            Some("style-src 'self'; script-src 'unsafe-inline'")
        ),
        [HtmlIssueKind::BlockedInlineAttribute]
    );
}

#[test]
fn issues_carry_their_line_and_a_bounded_excerpt() {
    let long = "x".repeat(120);
    let page = format!("<main>\n  <p>ok</p>\n  <p>&lt;b {long}</p>\n<i>&lt;i</i>   \n</main>");
    let issues = html_issues(&page, None);
    assert_eq!(issues.len(), 2, "{issues:?}");
    assert_eq!(issues[0].line(), 3);
    assert_eq!(issues[0].excerpt().chars().count(), 80);
    assert!(issues[0].excerpt().starts_with("&lt;b xxx"));
    assert_eq!(issues[1].line(), 4);
    assert_eq!(issues[1].excerpt(), "&lt;i</i>");
    assert_eq!(
        issues[1].to_string(),
        "line 4: escaped twice: visitors see the markup or entity: `&lt;i</i>`"
    );
}

#[test]
fn each_kind_says_what_is_wrong() {
    for (page, words) in [
        ("<p>&lt;b</p>", "escaped twice"),
        ("<p a=b\"c>", "malformed markup"),
        ("<p hidden=\"false\">", "\"false\" is still on"),
        ("<script>x</script>", "CSP nonce is blocked"),
        ("<p style=\"x\">", "nonces do not apply to attributes"),
    ] {
        let issues = html_issues(page, Some(STRICT));
        assert_eq!(issues.len(), 1, "{page}");
        let text = issues[0].to_string();
        assert!(text.starts_with("line 1: "), "{text}");
        assert!(text.contains(words), "{text}");
    }
}

#[test]
fn tokenizer_helpers_handle_short_and_odd_input() {
    assert!(opens_tag("<a"));
    assert!(opens_tag("</a"));
    assert!(!opens_tag("<"));
    assert!(!opens_tag("</"));
    assert!(!opens_tag("< a"));
    assert!(!opens_tag("</ a"));
    assert!(!opens_tag("a<b"));
    assert_eq!(next_markup("ab<c"), Some(2));
    assert_eq!(next_markup("ab<!x"), Some(2));
    assert_eq!(next_markup("ab<?x"), Some(2));
    assert_eq!(next_markup("<ab < c"), None);
    assert_eq!(skip(b"aab", 0, |byte| byte == b'a'), 2);
    assert_eq!(skip(b"aa", 0, |byte| byte == b'a'), 2);
    assert_eq!(skip(b"aa", 5, |byte| byte == b'a'), 5);
    assert_eq!(closing_tag("a</STYLE>", "style"), Some(1));
    assert_eq!(closing_tag("a</style", "style"), Some(1));
    assert_eq!(closing_tag("</style\n>", "style"), Some(0));
    assert_eq!(closing_tag("</style/>", "style"), Some(0));
    assert_eq!(closing_tag("</styles>", "style"), None);
    assert!(escaped_element(b"a"));
    assert!(escaped_element(b"/Span&gt;"));
    assert!(escaped_element(b"p/"));
    assert!(escaped_element(b"td\tx"));
    assert!(!escaped_element(b""));
    assert!(!escaped_element(b"/"));
    assert!(!escaped_element(b"pre-x"));
    assert!(!escaped_element(b"p&amp;"));
    assert!(numeric_reference("#1;"));
    assert!(numeric_reference("#xFf;"));
    assert!(!numeric_reference("1;"));
    assert!(!numeric_reference("#xg;"));
}
