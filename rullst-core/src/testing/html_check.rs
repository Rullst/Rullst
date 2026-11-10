//! Checks of rendered HTML for mistakes browsers repair or block without a
//! visible error: escaped markup, broken attribute quoting, boolean
//! attributes set to `"false"`, and inline code the response's Content
//! Security Policy refuses.
//!
//! Like [`normalize_html`](super::normalize_html) this is a small tokenizer,
//! not an HTML parser: it reads tags, attributes, comments and the raw-text
//! elements, which is what these checks need.

use super::csp_policy::{Inline, Policies};
use std::fmt;

/// Elements whose text may show escaped markup on purpose (code samples).
const VERBATIM_ELEMENTS: [&str; 4] = ["code", "kbd", "pre", "samp"];
/// Elements whose content is not markup and ends only at their end tag.
const RAW_TEXT_ELEMENTS: [&str; 4] = ["script", "style", "textarea", "title"];
/// HTML attributes that are on whenever present, whatever their value.
const BOOLEAN_ATTRIBUTES: [&str; 24] = [
    "allowfullscreen",
    "async",
    "autofocus",
    "autoplay",
    "checked",
    "controls",
    "default",
    "defer",
    "disabled",
    "formnovalidate",
    "hidden",
    "inert",
    "ismap",
    "itemscope",
    "loop",
    "multiple",
    "muted",
    "nomodule",
    "novalidate",
    "open",
    "playsinline",
    "readonly",
    "required",
    "selected",
];
/// Element names whose escaped tag (`&lt;div`) in text means markup was
/// escaped again.
const ELEMENT_NAMES: [&str; 76] = [
    "a",
    "abbr",
    "article",
    "aside",
    "audio",
    "b",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "caption",
    "code",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "mark",
    "meta",
    "nav",
    "ol",
    "optgroup",
    "option",
    "p",
    "picture",
    "pre",
    "script",
    "section",
    "select",
    "small",
    "source",
    "span",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "svg",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "tr",
    "u",
    "ul",
];
/// Characters [`HtmlIssue::excerpt`] keeps.
const EXCERPT_CHARS: usize = 80;

/// What an [`HtmlIssue`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HtmlIssueKind {
    /// An escaped element tag or comment in text, such as `&lt;span class`
    /// or `&lt;/div&gt;`, or an escaped entity such as `&amp;lt;` in text or
    /// an attribute: a value was escaped twice, so visitors read the tags or
    /// entities. Text inside `code`, `kbd`, `pre`, `samp` and `textarea` is
    /// not checked. Text that shows a tag on purpose elsewhere is reported
    /// too.
    DoubleEscaped,
    /// Markup that does not parse as written: a quote, `<` or `=` in an
    /// attribute name, a quote, `<`, `=` or backtick in an unquoted value, a
    /// quoted value directly followed by another character (often a `"`
    /// inside a value), `name=` without a value, or a tag, comment or
    /// raw-text element that never ends.
    MalformedMarkup,
    /// A boolean attribute such as `disabled="false"`: presence turns it on
    /// whatever the value, so the control is disabled.
    FalseBooleanAttribute,
    /// An inline `<script>` (without `src`) or `<style>` that the response's
    /// policy refuses: it lacks the response's CSP nonce.
    MissingNonce,
    /// A `style` or `on*` attribute that the response's policy refuses.
    /// Nonces do not apply to attributes.
    BlockedInlineAttribute,
}

impl HtmlIssueKind {
    fn describe(self) -> &'static str {
        match self {
            Self::DoubleEscaped => "escaped twice: visitors see the markup or entity",
            Self::MalformedMarkup => {
                "malformed markup: a browser reads it differently than written"
            }
            Self::FalseBooleanAttribute => {
                "boolean attribute set to \"false\" is still on (write `name?={flag}`)"
            }
            Self::MissingNonce => {
                "inline <script> or <style> without the response's CSP nonce is blocked"
            }
            Self::BlockedInlineAttribute => {
                "style or on* attribute is blocked by the response's CSP (nonces do not apply to attributes)"
            }
        }
    }
}

/// One problem [`html_issues`] found, with the 1-based line where it starts
/// and the start of that line's text from there.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HtmlIssue {
    kind: HtmlIssueKind,
    line: usize,
    excerpt: String,
}

impl HtmlIssue {
    /// What was found.
    pub fn kind(&self) -> HtmlIssueKind {
        self.kind
    }

    /// The 1-based line of the HTML where the problem starts.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Up to 80 characters of the HTML from where the problem starts, to the
    /// end of that line.
    pub fn excerpt(&self) -> &str {
        &self.excerpt
    }
}

impl fmt::Display for HtmlIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "line {}: {}: `{}`",
            self.line,
            self.kind.describe(),
            self.excerpt
        )
    }
}

/// The problems a browser would silently repair or block in `html`, in
/// document order (see [`HtmlIssueKind`]).
///
/// `content_security_policy` is the response's `Content-Security-Policy`
/// value (several headers joined with `,`). With it, inline `<script>` and
/// `<style>` elements must carry a nonce it lists, and `style`/`on*`
/// attributes must be allowed by it; without it those checks are skipped.
/// A hash source is assumed to match, since content hashes are not
/// computed.
///
/// ```
/// use rullst_core::testing::{HtmlIssueKind, html_issues};
///
/// let csp = "default-src 'self'; script-src 'self' 'nonce-abc'";
/// let page = "<p title=\"Say \"hi\"\">&lt;b&gt;</p><script>run()</script>";
/// let kinds: Vec<_> = html_issues(page, Some(csp)).iter().map(|issue| issue.kind()).collect();
/// assert_eq!(
///     kinds,
///     [HtmlIssueKind::MalformedMarkup, HtmlIssueKind::DoubleEscaped, HtmlIssueKind::MissingNonce]
/// );
/// assert!(html_issues("<script nonce=\"abc\">run()</script>", Some(csp)).is_empty());
/// ```
pub fn html_issues(html: &str, content_security_policy: Option<&str>) -> Vec<HtmlIssue> {
    let mut scanner = Scanner {
        html,
        policies: content_security_policy.map(Policies::parse),
        issues: Vec::new(),
        verbatim: 0,
    };
    let mut index = 0;
    while index < html.len() {
        index = scanner.step(index);
    }
    scanner.issues
}

struct Scanner<'a> {
    html: &'a str,
    policies: Option<Policies>,
    issues: Vec<HtmlIssue>,
    /// Open verbatim elements around the current position.
    verbatim: usize,
}

/// One attribute and the byte offset of its name.
struct Attribute<'a> {
    name: &'a str,
    value: Option<&'a str>,
    start: usize,
}

/// One start or end tag.
struct Tag<'a> {
    name: &'a str,
    closing: bool,
    attributes: Vec<Attribute<'a>>,
    /// The offset of the first malformed attribute, or of the tag when it
    /// never ends: one defect often breaks the attributes after it too.
    problem: Option<usize>,
    /// The offset just after `>`, or the document length.
    end: usize,
    complete: bool,
    self_closing: bool,
}

impl Tag<'_> {
    fn has(&self, name: &str) -> bool {
        self.attributes
            .iter()
            .any(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
            .and_then(|attribute| attribute.value)
    }
}

impl Scanner<'_> {
    /// Reads the comment, declaration, tag or text run at `start` and
    /// returns where the next one begins (always after `start`).
    fn step(&mut self, start: usize) -> usize {
        let rest = &self.html[start..];
        for (open, close) in [("<!--", "-->"), ("<![CDATA[", "]]>")] {
            if rest.starts_with(open) {
                return self.skip_past(start, open.len(), close);
            }
        }
        if rest.starts_with("<!") || rest.starts_with("<?") {
            return self.skip_past(start, 2, ">");
        }
        if opens_tag(rest) {
            return self.tag(start);
        }
        let end = next_markup(rest).map_or(self.html.len(), |offset| start + offset);
        self.text(start, end);
        end
    }

    fn skip_past(&mut self, start: usize, opener: usize, close: &str) -> usize {
        match self.html[start + opener..].find(close) {
            Some(offset) => start + opener + offset + close.len(),
            None => {
                self.report(HtmlIssueKind::MalformedMarkup, start);
                self.html.len()
            }
        }
    }

    fn tag(&mut self, start: usize) -> usize {
        let tag = parse_tag(self.html, start);
        if let Some(problem) = tag.problem {
            self.report(HtmlIssueKind::MalformedMarkup, problem);
        }
        let name = tag.name.to_ascii_lowercase();
        let verbatim = VERBATIM_ELEMENTS.contains(&name.as_str());
        if tag.closing {
            if verbatim {
                self.verbatim = self.verbatim.saturating_sub(1);
            }
            return tag.end;
        }
        for attribute in &tag.attributes {
            self.attribute(attribute);
        }
        if verbatim && !tag.self_closing {
            self.verbatim += 1;
        }
        if !tag.complete || !RAW_TEXT_ELEMENTS.contains(&name.as_str()) {
            return tag.end;
        }
        let Some(length) = closing_tag(&self.html[tag.end..], &name) else {
            self.report(HtmlIssueKind::MalformedMarkup, start);
            return self.html.len();
        };
        let content = &self.html[tag.end..tag.end + length];
        match name.as_str() {
            "script"
                if !tag.has("src") && !content.is_empty() && runs_as_script(tag.value("type")) =>
            {
                self.inline_element(Inline::ScriptElement, &tag, start);
            }
            "style" if !content.is_empty() => {
                self.inline_element(Inline::StyleElement, &tag, start)
            }
            "title" => self.text(tag.end, tag.end + length),
            _ => {}
        }
        // The end tag is read as the next step.
        tag.end + length
    }

    fn attribute(&mut self, attribute: &Attribute<'_>) {
        let name = attribute.name.to_ascii_lowercase();
        if let Some(value) = attribute.value {
            if BOOLEAN_ATTRIBUTES.contains(&name.as_str())
                && value.trim().eq_ignore_ascii_case("false")
            {
                self.report(HtmlIssueKind::FalseBooleanAttribute, attribute.start);
            }
            if double_escaped_entity(value).is_some() {
                self.report(HtmlIssueKind::DoubleEscaped, attribute.start);
            }
        }
        let inline = if name == "style" {
            Some(Inline::StyleAttribute)
        } else if is_event_handler(&name) {
            Some(Inline::ScriptAttribute)
        } else {
            None
        };
        let blocked = inline.is_some_and(|inline| {
            self.policies
                .as_ref()
                .is_some_and(|policies| !policies.allows(inline, None))
        });
        if blocked {
            self.report(HtmlIssueKind::BlockedInlineAttribute, attribute.start);
        }
    }

    fn inline_element(&mut self, inline: Inline, tag: &Tag<'_>, start: usize) {
        let blocked = self
            .policies
            .as_ref()
            .is_some_and(|policies| !policies.allows(inline, tag.value("nonce")));
        if blocked {
            self.report(HtmlIssueKind::MissingNonce, start);
        }
    }

    fn text(&mut self, start: usize, end: usize) {
        if self.verbatim > 0 {
            return;
        }
        if let Some(offset) = escaped_markup(&self.html[start..end]) {
            self.report(HtmlIssueKind::DoubleEscaped, start + offset);
        }
    }

    fn report(&mut self, kind: HtmlIssueKind, offset: usize) {
        let before = &self.html[..offset];
        let excerpt = self.html[offset..]
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(EXCERPT_CHARS)
            .collect::<String>();
        self.issues.push(HtmlIssue {
            kind,
            line: before.matches('\n').count() + 1,
            excerpt: excerpt.trim_end().to_owned(),
        });
    }
}

/// `<` followed by a letter, or `</` followed by a letter, opens a tag.
fn opens_tag(text: &str) -> bool {
    let bytes = text.as_bytes();
    let name = if bytes.get(1) == Some(&b'/') { 2 } else { 1 };
    bytes.first() == Some(&b'<') && bytes.get(name).is_some_and(u8::is_ascii_alphabetic)
}

/// The offset of the next tag, comment or declaration after the first byte.
fn next_markup(text: &str) -> Option<usize> {
    let first = text.chars().next()?.len_utf8();
    // `<` is ASCII, so each match is a character boundary.
    text[first..]
        .match_indices('<')
        .map(|(index, _)| first + index)
        .find(|index| {
            let rest = &text[*index..];
            opens_tag(rest) || rest.starts_with("<!") || rest.starts_with("<?")
        })
}

/// The first byte at or after `index` for which `keep` is false.
fn skip(bytes: &[u8], index: usize, keep: impl Fn(u8) -> bool) -> usize {
    bytes.get(index..).map_or(index, |rest| {
        index + rest.iter().take_while(|byte| keep(**byte)).count()
    })
}

fn parse_tag(html: &str, start: usize) -> Tag<'_> {
    let bytes = html.as_bytes();
    let closing = bytes.get(start + 1) == Some(&b'/');
    let name_start = start + 1 + usize::from(closing);
    let mut index = skip(bytes, name_start, |byte| {
        !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'>')
    });
    let mut tag = Tag {
        name: &html[name_start..index],
        closing,
        attributes: Vec::new(),
        problem: None,
        end: html.len(),
        complete: false,
        self_closing: false,
    };
    loop {
        index = skip(bytes, index, |byte| {
            byte.is_ascii_whitespace() || byte == b'/'
        });
        match bytes.get(index) {
            None => {
                tag.problem.get_or_insert(start);
                return tag;
            }
            Some(b'>') => {
                tag.self_closing = bytes.get(index - 1) == Some(&b'/');
                tag.end = index + 1;
                tag.complete = true;
                return tag;
            }
            Some(_) => index = parse_attribute(html, index, &mut tag),
        }
    }
}

/// Reads the attribute at `start` into `tag` and returns where it ends.
fn parse_attribute<'a>(html: &'a str, start: usize, tag: &mut Tag<'a>) -> usize {
    let bytes = html.as_bytes();
    // A leading `=` belongs to the name, as browsers read it.
    let name_end = skip(bytes, start + 1, |byte| {
        !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'>' | b'=')
    });
    let name = &html[start..name_end];
    if name
        .bytes()
        .any(|byte| matches!(byte, b'"' | b'\'' | b'<' | b'='))
    {
        tag.problem.get_or_insert(start);
    }
    let mut index = skip(bytes, name_end, |byte| byte.is_ascii_whitespace());
    let mut value = None;
    if bytes.get(index) == Some(&b'=') {
        index = skip(bytes, index + 1, |byte| byte.is_ascii_whitespace());
        match bytes.get(index) {
            Some(&quote @ (b'"' | b'\'')) => {
                let begin = index + 1;
                let end = skip(bytes, begin, |byte| byte != quote);
                value = Some(&html[begin..end]);
                // An unclosed value runs to the end; the tag reports it.
                index = (end + 1).min(html.len());
                let joined = bytes.get(index).is_some_and(|byte| {
                    !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'>')
                });
                if joined {
                    tag.problem.get_or_insert(start);
                }
            }
            Some(b'>') | None => {
                tag.problem.get_or_insert(start);
                value = Some("");
            }
            Some(_) => {
                let begin = index;
                index = skip(bytes, index, |byte| {
                    !byte.is_ascii_whitespace() && byte != b'>'
                });
                let unquoted = &html[begin..index];
                if unquoted
                    .bytes()
                    .any(|byte| matches!(byte, b'"' | b'\'' | b'<' | b'=' | b'`'))
                {
                    tag.problem.get_or_insert(start);
                }
                value = Some(unquoted);
            }
        }
    }
    tag.attributes.push(Attribute { name, value, start });
    index
}

/// The length of a raw-text element's content: the offset of `</name`
/// (any letter case) followed by whitespace, `/`, `>` or the end.
fn closing_tag(text: &str, name: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    let needle = format!("</{name}");
    lower
        .match_indices(&needle)
        .map(|(start, _)| start)
        .find(|start| {
            lower
                .as_bytes()
                .get(start + needle.len())
                .is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b'/' | b'>'))
        })
}

/// Whether a `<script type>` makes the browser run its content.
fn runs_as_script(kind: Option<&str>) -> bool {
    let Some(kind) = kind.map(|kind| kind.trim().to_ascii_lowercase()) else {
        return true;
    };
    let essence = kind.split(';').next().unwrap_or_default().trim();
    kind.is_empty()
        || matches!(essence, "module" | "importmap" | "speculationrules")
        || ["javascript", "ecmascript", "jscript", "livescript"]
            .into_iter()
            .any(|language| essence.contains(language))
}

/// `on` followed by lowercase letters, as `onclick`.
fn is_event_handler(name: &str) -> bool {
    name.strip_prefix("on").is_some_and(|event| {
        !event.is_empty() && event.bytes().all(|byte| byte.is_ascii_lowercase())
    })
}

/// The offset of the first `&lt;` that starts an escaped element tag or
/// comment, or of the first [`double_escaped_entity`].
fn escaped_markup(text: &str) -> Option<usize> {
    let tag = text
        .match_indices("&lt;")
        .map(|(start, _)| start)
        .find(|start| {
            let after = &text.as_bytes()[start + 4..];
            after.starts_with(b"!--") || escaped_element(after)
        });
    match (tag, double_escaped_entity(text)) {
        (Some(tag), Some(entity)) => Some(tag.min(entity)),
        (tag, entity) => tag.or(entity),
    }
}

/// Whether `after` (the text after `&lt;`) begins with an optional `/`, an
/// HTML element name and then whitespace, `/`, `&gt;` or the end: `&lt;div&gt;`
/// or `&lt;a href`, but not `&lt;3` or `&lt;Rullst & co&gt;`.
fn escaped_element(after: &[u8]) -> bool {
    let name = after.strip_prefix(b"/").unwrap_or(after);
    let length = name
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric())
        .count();
    let (word, rest) = name.split_at(length);
    ELEMENT_NAMES
        .iter()
        .any(|element| element.as_bytes().eq_ignore_ascii_case(word))
        && (rest.is_empty()
            || rest.starts_with(b"/")
            || rest.starts_with(b"&gt;")
            || rest.first().is_some_and(u8::is_ascii_whitespace))
}

/// The offset of the first `&amp;` that escapes an entity again, such as
/// `&amp;lt;` or `&amp;#x27;`.
fn double_escaped_entity(text: &str) -> Option<usize> {
    text.match_indices("&amp;")
        .map(|(start, _)| start)
        .find(|start| {
            let after = &text[start + 5..];
            ["lt;", "gt;", "amp;", "quot;", "apos;"]
                .into_iter()
                .any(|entity| after.starts_with(entity))
                || numeric_reference(after)
        })
}

/// `#` and decimal digits, or `#x` and hexadecimal digits, then `;`.
fn numeric_reference(text: &str) -> bool {
    let Some(reference) = text.strip_prefix('#') else {
        return false;
    };
    let (digits, radix) = match reference.strip_prefix(['x', 'X']) {
        Some(hex) => (hex, 16),
        None => (reference, 10),
    };
    let length = digits
        .chars()
        .take_while(|character| character.is_digit(radix))
        .count();
    length > 0 && digits[length..].starts_with(';')
}

#[cfg(test)]
#[path = "html_check_tests.rs"]
mod tests;
