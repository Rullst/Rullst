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
    // Occurrences inside an extracted body belong to that body.
    let mut consumed = 0;
    for (start, keyword) in source.match_indices("html!") {
        if start < consumed {
            continue;
        }
        let preceded = source[..start]
            .chars()
            .next_back()
            .is_some_and(|before| before.is_alphanumeric() || before == '_');
        if preceded {
            continue;
        }
        let after = &source[start + keyword.len()..];
        let open = source.len() - after.trim_start().len();
        let Some(&delimiter) = bytes.get(open) else {
            break;
        };
        if !matches!(delimiter, b'{' | b'(' | b'[') {
            continue;
        }
        if let Some(close) = matching_close(bytes, open) {
            bodies.push((open + 1, &source[open + 1..close]));
            consumed = close;
        }
    }
    bodies
}

/// The index of the bracket closing the one at `open`; string literals are skipped.
fn matching_close(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, &byte) in bytes.iter().enumerate().skip(open) {
        if escaped {
            escaped = false;
        } else if in_string {
            match byte {
                b'\\' => escaped = true,
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
    }
    None
}

/// Every tag in `text`; `<!-- -->` comments and `<!DOCTYPE>` are skipped.
pub(super) fn tags(text: &str) -> Vec<Tag> {
    let bytes = text.as_bytes();
    let mut tags = Vec::new();
    // Openers before `resume` lie inside a comment or a parsed tag.
    let mut resume = 0;
    for (index, _) in text.match_indices('<') {
        if index < resume {
            continue;
        }
        if text[index..].starts_with("<!--") {
            resume = text[index..]
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
            continue;
        }
        let (attributes, end) = attributes(bytes, name_end);
        tags.push(Tag {
            name: text[name_start..name_end].to_ascii_lowercase(),
            closing,
            attributes,
            offset: index,
        });
        resume = end;
    }
    tags
}

/// The length of the run of bytes from `index` that `include` accepts.
fn run_length(bytes: &[u8], index: usize, include: impl Fn(u8) -> bool) -> usize {
    bytes.get(index..).map_or(0, |rest| {
        rest.iter().take_while(|byte| include(**byte)).count()
    })
}

/// The end of an attribute value starting at `index`: quoted, `{...}` or bare.
fn value_end(bytes: &[u8], index: usize) -> usize {
    match bytes.get(index) {
        Some(&quote @ (b'"' | b'\'')) => {
            (index + 2 + run_length(bytes, index + 1, |byte| byte != quote)).min(bytes.len())
        }
        Some(b'{') => matching_close(bytes, index).map_or(bytes.len(), |end| end + 1),
        _ => {
            index
                + run_length(bytes, index, |byte| {
                    !byte.is_ascii_whitespace() && byte != b'>'
                })
        }
    }
}

/// Attributes from `start` to the closing `>`; values may be quoted or `{...}`.
/// Whitespace, `/` and other bytes that start no name are skipped.
fn attributes(bytes: &[u8], start: usize) -> (Vec<(String, Option<String>)>, usize) {
    let mut attributes = Vec::new();
    let text = |from: usize, to: usize| String::from_utf8_lossy(&bytes[from..to]).into_owned();
    // Bytes before `resume` belong to an attribute already read.
    let mut resume = start;
    for (index, &byte) in bytes.iter().enumerate().skip(start) {
        if index < resume {
            continue;
        }
        match byte {
            b'>' => return (attributes, index + 1),
            b'{' => resume = matching_close(bytes, index).map_or(bytes.len(), |end| end + 1),
            _ => {
                resume = index
                    + run_length(bytes, index, |byte| {
                        !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'>' | b'/')
                    });
                let name = text(index, resume).to_ascii_lowercase();
                let mut value = None;
                if bytes.get(resume) == Some(&b'=') {
                    let value_start = resume
                        + 1
                        + run_length(bytes, resume + 1, |byte| byte.is_ascii_whitespace());
                    resume = value_end(bytes, value_start);
                    value = Some(
                        text(value_start, resume)
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
