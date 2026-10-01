//! Production-source views of a Rust file for the bounded audit heuristics.
//!
//! Each top-level item annotated `#[cfg(test)]` (for example `mod tests;`, an
//! inline test module or a test-only `use`) is removed on its own; code after
//! it is still scanned. Removed bytes become spaces and newlines are kept, so
//! reported line numbers stay those of the file.

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

pub(super) struct AuditSource {
    /// The file without its top-level `#[cfg(test)]` items.
    pub(super) production: String,
    /// `production` without Rust comments or doc attributes, for guard
    /// evidence. String literals (generator templates) are retained.
    pub(super) code: String,
}

/// Splits `content` into its production and comment-free views.
///
/// A file that cannot be tokenized is returned unchanged in `production`, so
/// the scan fails closed by covering its test code too.
pub(super) fn audit_source(content: &str) -> AuditSource {
    let Some(views) = tokenized_views(content) else {
        return AuditSource {
            production: content.to_string(),
            code: without_comment_lines(content),
        };
    };
    views
}

/// The production view only, for scanners that need no guard evidence.
pub(super) fn production_source(content: &str) -> String {
    audit_source(content).production
}

fn tokenized_views(content: &str) -> Option<AuditSource> {
    let tokens = content.parse::<TokenStream>().ok()?;
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let mut test_items = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if let Some(after_attribute) = cfg_test_attribute(&tokens, index) {
            let last = item_end(&tokens, skip_outer_attributes(&tokens, after_attribute));
            let start = tokens.get(index)?.span().byte_range().start;
            let end = tokens.get(last)?.span().byte_range().end;
            test_items.push(start..end);
            index = last + 1;
        } else {
            index += 1;
        }
    }

    let mut kept = vec![false; content.len()];
    mark_code_tokens(tokens, &mut kept);
    let production = blank(content, |offset| {
        test_items.iter().any(|item| item.contains(&offset))
    })?;
    let code = blank(&production, |offset| {
        !kept.get(offset).copied().unwrap_or(false)
    })?;
    Some(AuditSource {
        production,
        code: without_comment_lines(&code),
    })
}

/// `#[cfg(test)]` at `index`; returns the index after the attribute.
fn cfg_test_attribute(tokens: &[TokenTree], index: usize) -> Option<usize> {
    let (TokenTree::Punct(hash), TokenTree::Group(attribute)) =
        (tokens.get(index)?, tokens.get(index + 1)?)
    else {
        return None;
    };
    if hash.as_char() != '#' || attribute.delimiter() != Delimiter::Bracket {
        return None;
    }
    let inner = attribute.stream().into_iter().collect::<Vec<_>>();
    let [TokenTree::Ident(cfg), TokenTree::Group(predicate)] = inner.as_slice() else {
        return None;
    };
    let predicate = predicate.stream().into_iter().collect::<Vec<_>>();
    let is_test = matches!(predicate.as_slice(), [TokenTree::Ident(test)] if test == "test");
    (cfg == "cfg" && is_test).then_some(index + 2)
}

/// Skips further outer attributes (including doc comments) of the same item.
fn skip_outer_attributes(tokens: &[TokenTree], mut index: usize) -> usize {
    while let (Some(TokenTree::Punct(hash)), Some(TokenTree::Group(group))) =
        (tokens.get(index), tokens.get(index + 1))
    {
        if hash.as_char() != '#' || group.delimiter() != Delimiter::Bracket {
            break;
        }
        index += 2;
    }
    index
}

/// Index of the last token of the item starting at `index`: its terminating
/// `;`, or its body when no `;` follows (`mod tests { .. }`, `fn f() { .. }`).
fn item_end(tokens: &[TokenTree], index: usize) -> usize {
    let mut cursor = index;
    while let Some(token) = tokens.get(cursor) {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == ';' => return cursor,
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                let terminated = matches!(
                    tokens.get(cursor + 1),
                    Some(TokenTree::Punct(next)) if next.as_char() == ';'
                );
                if !terminated {
                    return cursor;
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    tokens.len().saturating_sub(1)
}

/// Marks the bytes of every token except doc attributes; comments and
/// whitespace are never tokens. Iterative, so nesting depth is not bounded by
/// the stack.
fn mark_code_tokens(tokens: Vec<TokenTree>, kept: &mut [bool]) {
    let mut pending = vec![tokens];
    while let Some(tokens) = pending.pop() {
        let mut index = 0;
        while index < tokens.len() {
            if let Some(after) = doc_attribute(&tokens, index) {
                index = after;
                continue;
            }
            let Some(token) = tokens.get(index) else {
                break;
            };
            match token {
                TokenTree::Group(group) => {
                    keep(kept, group.span_open().byte_range());
                    keep(kept, group.span_close().byte_range());
                    pending.push(group.stream().into_iter().collect());
                }
                other => keep(kept, other.span().byte_range()),
            }
            index += 1;
        }
    }
}

/// `#[doc ..]` or `#![doc ..]` (what `///` and `//!` comments lex to).
fn doc_attribute(tokens: &[TokenTree], index: usize) -> Option<usize> {
    let TokenTree::Punct(hash) = tokens.get(index)? else {
        return None;
    };
    if hash.as_char() != '#' {
        return None;
    }
    let mut next = index + 1;
    if let Some(TokenTree::Punct(bang)) = tokens.get(next)
        && bang.as_char() == '!'
        && bang.spacing() == Spacing::Alone
    {
        next += 1;
    }
    let TokenTree::Group(attribute) = tokens.get(next)? else {
        return None;
    };
    let is_doc = attribute.delimiter() == Delimiter::Bracket
        && matches!(attribute.stream().into_iter().next(), Some(TokenTree::Ident(name)) if name == "doc");
    is_doc.then_some(next + 1)
}

fn keep(kept: &mut [bool], range: std::ops::Range<usize>) {
    if let Some(bytes) = kept.get_mut(range) {
        bytes.fill(true);
    }
}

/// Replaces the selected bytes, except line breaks, with spaces.
fn blank(content: &str, mut remove: impl FnMut(usize) -> bool) -> Option<String> {
    let bytes = content
        .bytes()
        .enumerate()
        .map(|(offset, byte)| {
            if byte != b'\n' && remove(offset) {
                b' '
            } else {
                byte
            }
        })
        .collect::<Vec<_>>();
    // Whole tokens and comments are removed, so no character is split; the
    // check keeps a surprising span from producing invalid text.
    String::from_utf8(bytes).ok()
}

/// Drops lines that are only a comment. Rust comments are already removed
/// from tokenized code, so this applies to template text in string literals
/// (and to the untokenized fallback).
fn without_comment_lines(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("* ") {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn only_the_test_item_is_removed_and_lines_are_preserved() {
        let source = "use x;\n#[cfg(test)]\nmod tests;\n\nfn routes() { get(\"/a/{id}\" => show) }\n#[cfg(test)]\nmod inline {\n    fn t() {}\n}\nfn after() {}\n";
        let views = audit_source(source);
        assert_eq!(views.production.lines().count(), source.lines().count());
        assert!(!views.production.contains("mod tests"));
        assert!(!views.production.contains("mod inline"));
        assert!(!views.production.contains("fn t()"));
        assert!(views.production.contains("get(\"/a/{id}\" => show)"));
        assert!(views.production.contains("fn after() {}"));
        assert_eq!(
            views.production.lines().nth(4),
            Some("fn routes() { get(\"/a/{id}\" => show) }")
        );
    }

    #[test]
    fn test_items_end_at_their_own_terminator() {
        let source = "#[cfg(test)]\n#[path = \"x_tests.rs\"]\n/// doc\nuse a::{b, c};\nconst LIMIT: [u8; 2] = [1, 2];\n#[cfg(test)]\nstatic X: S = S { a: 1 };\nfn kept() {}\n#[cfg(not(test))]\nfn production() {}\n";
        let production = production_source(source);
        assert!(!production.contains("use a::"));
        assert!(production.contains("const LIMIT"));
        assert!(!production.contains("static X"));
        assert!(production.contains("fn kept()"));
        assert!(production.contains("fn production()"));
    }

    #[test]
    fn cfg_test_inside_a_string_or_comment_is_not_an_attribute() {
        let source =
            "const T: &str = \"\n#[cfg(test)]\nmod tests {}\n\";\n// #[cfg(test)]\nfn after() {}\n";
        assert_eq!(production_source(source), source);
    }

    #[test]
    fn code_view_drops_comments_and_doc_attributes_but_keeps_literals() {
        let source = "/// RbacGuard::authorize_owner_or_role\n// TODO RequireRoleLayer\n/* protect_router( */\nfn f() { call(); } // trailing protect_router(\nconst T: &str = r#\"\n// template comment RbacGuard::authorize(\nlet x = RbacGuard::authorize(ctx);\n\"#;\n";
        let code = audit_source(source).code;
        assert!(!code.contains("authorize_owner_or_role"));
        assert!(!code.contains("RequireRoleLayer"));
        assert!(!code.contains("protect_router("));
        assert!(code.contains("let x = RbacGuard::authorize(ctx);"));
        assert!(!code.contains("template comment"));
        assert!(code.contains("fn f() { call(); }"));
    }

    #[test]
    fn untokenizable_files_are_scanned_whole() {
        let source = "fn broken( {\n#[cfg(test)]\nmod tests { get(\"/a/{id}\" => show) }\n";
        assert_eq!(audit_source(source).production, source);
    }
}
