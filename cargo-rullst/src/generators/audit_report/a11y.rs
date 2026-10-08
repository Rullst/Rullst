//! Accessibility heuristics over `html!` invocations and HTML templates.
//!
//! A small tag scanner reads `<tag attr="..." attr={expr}>` markup; it does
//! not render templates, evaluate expressions or follow includes, so a
//! control labelled by included markup is reported for review.

use std::collections::HashSet;
use std::path::Path;

use super::catalog::{FORM_LABELS, HTML_LANG, IMG_ALT};
use super::model::{Check, Finding};
use super::sources::{ProjectSources, read_bounded, relative};
use crate::generators::source_walk::files_with_extensions;

/// Template extensions read under `src/` and `templates/`.
pub(super) const TEMPLATE_EXTENSIONS: [&str; 6] = ["html", "htm", "tera", "hbs", "jinja", "j2"];

/// One piece of markup and where it starts.
pub(super) struct Markup {
    pub file: String,
    pub first_line: usize,
    pub text: String,
}

/// One opening or closing tag.
#[derive(Debug)]
pub(super) struct Tag {
    pub name: String,
    pub closing: bool,
    pub attributes: Vec<(String, Option<String>)>,
    pub offset: usize,
}

impl Tag {
    fn attribute(&self, name: &str) -> Option<Option<&str>> {
        self.attributes
            .iter()
            .find(|(attribute, _)| attribute == name)
            .map(|(_, value)| value.as_deref())
    }

    fn has(&self, name: &str) -> bool {
        self.attribute(name).is_some()
    }
}

/// The bodies of `html!` invocations in `source`, with their byte offsets.
pub(super) fn html_macro_bodies(source: &str) -> Vec<(usize, &str)> {
    let bytes = source.as_bytes();
    let mut bodies = Vec::new();
    let mut search = 0;
    while let Some(found) = source[search..].find("html!") {
        let start = search + found;
        search = start + "html!".len();
        let preceded = source[..start]
            .chars()
            .next_back()
            .is_some_and(|before| before.is_alphanumeric() || before == '_');
        if preceded {
            continue;
        }
        let open = search + (source.len() - search - source[search..].trim_start().len());
        let Some(&delimiter) = bytes.get(open) else {
            break;
        };
        if !matches!(delimiter, b'{' | b'(' | b'[') {
            continue;
        }
        if let Some(close) = matching_close(bytes, open) {
            bodies.push((open + 1, &source[open + 1..close]));
            search = close;
        }
    }
    bodies
}

/// The index of the bracket closing the one at `open`; string literals are skipped.
fn matching_close(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    let mut in_string = false;
    while let Some(&byte) = bytes.get(index) {
        if in_string {
            match byte {
                b'\\' => index += 1,
                b'"' => in_string = false,
                _ => {}
            }
        } else {
            match byte {
                b'"' => in_string = true,
                b'{' | b'(' | b'[' => depth += 1,
                b'}' | b')' | b']' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }
    None
}

/// Every tag in `text`; `<!-- -->` comments and `<!DOCTYPE>` are skipped.
pub(super) fn tags(text: &str) -> Vec<Tag> {
    let bytes = text.as_bytes();
    let mut tags = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        if text[index..].starts_with("<!--") {
            index = text[index..]
                .find("-->")
                .map_or(bytes.len(), |end| index + end + 3);
            continue;
        }
        let closing = bytes.get(index + 1) == Some(&b'/');
        let name_start = index + 1 + usize::from(closing);
        let name_end = bytes[name_start.min(bytes.len())..]
            .iter()
            .position(|byte| !(byte.is_ascii_alphanumeric() || *byte == b'-'))
            .map_or(bytes.len(), |end| name_start + end);
        if name_end == name_start || !bytes[name_start].is_ascii_alphabetic() {
            index += 1;
            continue;
        }
        let (attributes, end) = attributes(bytes, name_end);
        tags.push(Tag {
            name: text[name_start..name_end].to_ascii_lowercase(),
            closing,
            attributes,
            offset: index,
        });
        index = end;
    }
    tags
}

/// Attributes from `start` to the closing `>`; values may be quoted or `{...}`.
fn attributes(bytes: &[u8], start: usize) -> (Vec<(String, Option<String>)>, usize) {
    let mut attributes = Vec::new();
    let mut index = start;
    let text = |from: usize, to: usize| String::from_utf8_lossy(&bytes[from..to]).into_owned();
    while index < bytes.len() {
        match bytes[index] {
            b'>' => return (attributes, index + 1),
            byte if byte.is_ascii_whitespace() || byte == b'/' => index += 1,
            b'{' => index = matching_close(bytes, index).map_or(bytes.len(), |end| end + 1),
            _ => {
                let name_start = index;
                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && !matches!(bytes[index], b'=' | b'>' | b'/')
                {
                    index += 1;
                }
                let name = text(name_start, index).to_ascii_lowercase();
                let mut value = None;
                if bytes.get(index) == Some(&b'=') {
                    index += 1;
                    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                        index += 1;
                    }
                    let value_start = index;
                    index = match bytes.get(index) {
                        Some(quote @ (b'"' | b'\'')) => bytes[index + 1..]
                            .iter()
                            .position(|byte| byte == quote)
                            .map_or(bytes.len(), |end| index + 1 + end + 1),
                        Some(b'{') => {
                            matching_close(bytes, index).map_or(bytes.len(), |end| end + 1)
                        }
                        _ => bytes[index..]
                            .iter()
                            .position(|byte| byte.is_ascii_whitespace() || *byte == b'>')
                            .map_or(bytes.len(), |end| index + end),
                    };
                    value = Some(
                        text(value_start, index)
                            .trim_matches(['"', '\''])
                            .to_string(),
                    );
                }
                if !name.is_empty() {
                    attributes.push((name, value));
                }
            }
        }
    }
    (attributes, bytes.len())
}

/// Markup from `html!` bodies in production Rust sources and from template
/// files under `src/` and `templates/`.
pub(super) fn collect(root: &Path, sources: &ProjectSources) -> (Vec<Markup>, Vec<String>) {
    let mut markup = Vec::new();
    for file in &sources.files {
        for (offset, body) in html_macro_bodies(&file.production) {
            markup.push(Markup {
                file: file.display.clone(),
                first_line: file.production[..offset].matches('\n').count() + 1,
                text: body.to_string(),
            });
        }
    }
    let mut incomplete = Vec::new();
    for directory in ["src", "templates"] {
        let walk = files_with_extensions(&root.join(directory), &TEMPLATE_EXTENSIONS);
        if let Some(reason) = walk.incomplete {
            incomplete.push(format!("{directory}: {reason}"));
        }
        for path in walk.files {
            if let Some(text) = read_bounded(&path) {
                markup.push(Markup {
                    file: relative(root, &path),
                    first_line: 1,
                    text,
                });
            }
        }
    }
    (markup, incomplete)
}

fn line_of(markup: &Markup, offset: usize) -> usize {
    markup.first_line + markup.text[..offset].matches('\n').count()
}

/// Controls that need a label; `type` values that never do are excluded.
fn needs_label(tag: &Tag) -> bool {
    match tag.name.as_str() {
        "select" | "textarea" => true,
        "input" => !matches!(
            tag.attribute("type")
                .flatten()
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("hidden" | "submit" | "button" | "reset" | "image")
        ),
        _ => false,
    }
}

/// The three accessibility checks over `markup`.
pub(super) fn checks(markup: &[Markup], incomplete: &[String]) -> Vec<Check> {
    if markup.is_empty() {
        let reason = "no html! invocation or template file was found";
        return vec![
            Check::not_checked(&IMG_ALT, reason),
            Check::not_checked(&FORM_LABELS, reason),
            Check::not_checked(&HTML_LANG, reason),
        ];
    }
    let (mut images, mut controls, mut pages) = (Vec::new(), Vec::new(), Vec::new());
    let (mut image_count, mut control_count, mut page_count) = (0usize, 0usize, 0usize);
    for piece in markup {
        let tags = tags(&piece.text);
        let labelled: HashSet<&str> = tags
            .iter()
            .filter(|tag| tag.name == "label" && !tag.closing)
            .filter_map(|tag| tag.attribute("for").flatten())
            .collect();
        let dynamic_for = labelled.iter().any(|value| value.starts_with('{'));
        let mut label_depth = 0usize;
        for tag in &tags {
            let at = Some(line_of(piece, tag.offset));
            match (tag.name.as_str(), tag.closing) {
                ("label", false) => label_depth += 1,
                ("label", true) => label_depth = label_depth.saturating_sub(1),
                ("img", false) => {
                    image_count += 1;
                    if !tag.has("alt") {
                        images.push(Finding::new(
                            piece.file.clone(),
                            at,
                            "`<img>` has no `alt` attribute",
                        ));
                    }
                }
                ("html", false) => {
                    page_count += 1;
                    if !tag.has("lang") {
                        pages.push(Finding::new(
                            piece.file.clone(),
                            at,
                            "`<html>` has no `lang` attribute",
                        ));
                    }
                }
                (_, false) if needs_label(tag) => {
                    control_count += 1;
                    let id = tag.attribute("id").flatten();
                    let associated = id.is_some_and(|id| {
                        labelled.contains(id) || (id.starts_with('{') && dynamic_for)
                    });
                    if label_depth == 0
                        && !associated
                        && !tag.has("aria-label")
                        && !tag.has("aria-labelledby")
                    {
                        controls.push(Finding::new(
                            piece.file.clone(),
                            at,
                            format!("`<{}>` has no associated label, wrapping label, aria-label or aria-labelledby", tag.name),
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    let scope = format!(
        "{} markup source(s) scanned{}. Static heuristic: rendered output, included templates and runtime attributes are not evaluated.",
        markup.len(),
        if incomplete.is_empty() {
            String::new()
        } else {
            format!("; incomplete walk: {}", incomplete.join("; "))
        },
    );
    vec![
        Check::from_findings(
            &IMG_ALT,
            images,
            format!("{image_count} <img> tag(s); {scope}"),
        ),
        Check::from_findings(
            &FORM_LABELS,
            controls,
            format!("{control_count} form control(s) needing a label; {scope}"),
        ),
        Check::from_findings(
            &HTML_LANG,
            pages,
            format!("{page_count} <html> element(s); {scope}"),
        ),
    ]
}

#[cfg(test)]
#[path = "a11y_tests.rs"]
mod tests;
