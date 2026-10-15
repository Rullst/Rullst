//! Message templates: text with `{name}` placeholders, `{{` and `}}` for
//! literal braces.

use std::collections::BTreeSet;

/// One piece of a parsed template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Segment {
    Text(String),
    Placeholder(String),
}

/// Parses a template, rejecting unbalanced braces and invalid names.
pub(crate) fn parse(template: &str) -> Result<Vec<Segment>, String> {
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.next_if_eq(&'{').is_some() => text.push('{'),
            '}' if chars.next_if_eq(&'}').is_some() => text.push('}'),
            '{' => {
                let mut name = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '}' {
                        closed = true;
                        break;
                    }
                    name.push(c);
                }
                if !closed {
                    return Err("has an unclosed `{`; write `{{` for a literal brace".into());
                }
                if !is_placeholder_name(&name) {
                    return Err(format!(
                        "has an invalid placeholder `{{{name}}}`: use a Rust identifier such as \
                         `{{name}}`, and `{{{{` for a literal brace"
                    ));
                }
                if !text.is_empty() {
                    segments.push(Segment::Text(std::mem::take(&mut text)));
                }
                segments.push(Segment::Placeholder(name));
            }
            '}' => return Err("has an unmatched `}`; write `}}` for a literal brace".into()),
            c => text.push(c),
        }
    }
    if !text.is_empty() {
        segments.push(Segment::Text(text));
    }
    Ok(segments)
}

/// The placeholder names a parsed template uses.
pub(crate) fn placeholders(segments: &[Segment]) -> BTreeSet<String> {
    segments
        .iter()
        .filter_map(|segment| match segment {
            Segment::Placeholder(name) => Some(name.clone()),
            Segment::Text(_) => None,
        })
        .collect()
}

/// A placeholder is an ASCII Rust identifier that is not a keyword, so
/// that `t!` can take it as a named argument.
fn is_placeholder_name(name: &str) -> bool {
    let mut chars = name.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    starts
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && syn::parse_str::<syn::Ident>(name).is_ok()
}
