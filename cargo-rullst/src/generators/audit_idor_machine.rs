//! Machine-route evidence for `// rullst-access: machine`: the method and
//! path of every `MachineEndpoint` registration in a crate's non-test code.
//!
//! A registration counts when its method is a `Method::POST|PUT|PATCH|DELETE`
//! path and its route is a string literal or a `const` of the same crate
//! holding one. Anything else (a computed path, a variable method) gives no
//! evidence, so the route keeps its finding.

use std::collections::HashMap;

const CONSTRUCTORS: [&str; 5] = [
    "bearer",
    "verified_bearer",
    "signed_webhook",
    "address_token",
    "mutual_tls",
];
const WRITE_METHODS: [&str; 4] = ["POST", "PUT", "PATCH", "DELETE"];

/// `const NAME: &str = "literal";` values of one crate. A name defined twice
/// with different values resolves to nothing.
#[derive(Default)]
pub(super) struct Constants(HashMap<String, Option<String>>);

impl Constants {
    pub(super) fn include(&mut self, code: &str) {
        let mut rest = code;
        while let Some(found) = find_word(rest, "const") {
            rest = &rest[found + "const".len()..];
            if let Some((name, value)) = string_constant(rest) {
                let entry = self.0.entry(name.to_owned()).or_insert(Some(value.clone()));
                if entry.as_ref() != Some(&value) {
                    *entry = None;
                }
            }
        }
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).and_then(Option::as_deref)
    }
}

/// `NAME: &str = "value";` (or `&'static str`) after the `const` keyword.
fn string_constant(text: &str) -> Option<(&str, String)> {
    let text = text.trim_start();
    let name_end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(text.len());
    let (name, text) = text.split_at(name_end);
    let text = text.trim_start().strip_prefix(':')?;
    let (kind, text) = text.split_once('=')?;
    if !matches!(
        kind.split_whitespace().collect::<String>().as_str(),
        "&str" | "&'staticstr"
    ) || name.is_empty()
    {
        return None;
    }
    let (value, text) = string_literal(text.trim_start())?;
    text.trim_start().starts_with(';').then_some((name, value))
}

/// A plain `"..."` literal without escapes, and the text after it.
fn string_literal(text: &str) -> Option<(String, &str)> {
    let body = text.strip_prefix('"')?;
    let end = body.find('"')?;
    let value = &body[..end];
    (!value.contains('\\')).then(|| (value.to_owned(), &body[end + 1..]))
}

/// The byte offset of `word` in `text` where it is not part of a longer
/// identifier.
fn find_word(text: &str, word: &str) -> Option<usize> {
    let mut start = 0;
    while let Some(offset) = text[start..].find(word) {
        let at = start + offset;
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        let ident = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if !ident(before) && !ident(after) {
            return Some(at);
        }
        start = at + word.len();
    }
    None
}

/// The registered `(METHOD, normalized path)` pairs of one crate.
#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct MachineRoutes(Vec<(&'static str, String)>);

impl MachineRoutes {
    /// Adds the registrations in `code`, resolving constant paths.
    pub(super) fn include(&mut self, code: &str, constants: &Constants) {
        let mut rest = code;
        while let Some(found) = rest.find("MachineEndpoint::") {
            rest = &rest[found + "MachineEndpoint::".len()..];
            if let Some(route) = registration(rest, constants) {
                self.0.push(route);
            }
        }
    }

    /// Whether every method of a route declaration is registered for `path`.
    pub(super) fn covers(&self, methods: &[&str], path: &str) -> bool {
        let path = normalized(path);
        !methods.is_empty()
            && methods.iter().all(|method| {
                self.0
                    .iter()
                    .any(|(registered, route)| registered == method && *route == path)
            })
    }

    /// Whether some method is registered for `path`.
    pub(super) fn declares(&self, path: &str) -> bool {
        let path = normalized(path);
        self.0.iter().any(|(_, route)| *route == path)
    }
}

/// `constructor(Method::X, path, …)` right after `MachineEndpoint::`.
fn registration(text: &str, constants: &Constants) -> Option<(&'static str, String)> {
    let open = text.find('(')?;
    if !CONSTRUCTORS.contains(&text[..open].trim()) {
        return None;
    }
    let mut arguments = arguments(&text[open + 1..]).into_iter();
    let method = arguments.next()?;
    let method = method.trim().rsplit("::").next()?;
    let method = *WRITE_METHODS.iter().find(|known| **known == method)?;
    let path = arguments.next()?;
    let path = path.trim().trim_start_matches('&').trim();
    let path = if path.starts_with('"') {
        let (value, rest) = string_literal(path)?;
        rest.trim().is_empty().then_some(value)?
    } else {
        let name = path.rsplit("::").next()?;
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return None;
        }
        constants.get(name)?.to_owned()
    };
    path.starts_with('/').then(|| (method, normalized(&path)))
}

/// The top-level comma-separated arguments up to the closing `)`, or none
/// when the call does not close.
fn arguments(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let (mut depth, mut start, mut quoted, mut escaped) = (0usize, 0, false, false);
    for (index, character) in text.char_indices() {
        if quoted {
            quoted = !(character == '"' && !escaped);
            escaped = character == '\\' && !escaped;
            continue;
        }
        match character {
            '"' => quoted = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth > 0 => depth -= 1,
            ')' => {
                found.push(&text[start..index]);
                return found;
            }
            ',' if depth == 0 => {
                found.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    // An unterminated call is not a registration.
    Vec::new()
}

/// Parameter segments (`{id}`, `{{id}}`, `:id`) become `{}`.
fn normalized(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if super::is_parameter_segment(segment) {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The upper-case HTTP methods of route calls (`post(`, `.put(`) in `text`;
/// `get` and `ws` are reported too, so a mixed route is never machine-only.
pub(super) fn route_methods(text: &str) -> Vec<&'static str> {
    [
        ("get(", "GET"),
        ("post(", "POST"),
        ("put(", "PUT"),
        ("patch(", "PATCH"),
        ("delete(", "DELETE"),
        ("ws(", "WS"),
    ]
    .into_iter()
    .filter(|(call, _)| find_call(text, call))
    .map(|(_, method)| method)
    .collect()
}

/// `call` in `text`, not preceded by an identifier character.
fn find_call(text: &str, call: &str) -> bool {
    text.match_indices(call).any(|(at, _)| {
        !text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "audit_idor_machine_tests.rs"]
mod tests;
