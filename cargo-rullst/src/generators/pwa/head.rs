//! Adds the PWA head tags to every page layout of one source file.
//!
//! Three layout shapes are recognized, the ones the starters use:
//! - an `html!` document: `{crate::pwa::head()}` goes before its `</head>`;
//! - a hand-written document in a string literal: the same tags, written with
//!   single-quoted attributes so that no string or raw-string delimiter and
//!   no `format!` brace is introduced, go before its `</head>`;
//! - Core's `render_page_with_lang(htmx, lang, title, content)`: it becomes
//!   `render_page_with_head(htmx, lang, title, crate::pwa::head(), content)`.
//!
//! A head that already links a manifest is left alone, so a second run
//! changes nothing. Comments are not tokens and are never edited.
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::{error::Error, io, ops::Range};

const HEAD_CALL: &str = "{crate::pwa::head()}";

/// The result of wiring one file.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Wiring {
    pub source: String,
    /// Layouts that received the tags now.
    pub wired: usize,
    /// Layouts that already link a manifest.
    pub present: usize,
}

/// The tags for a string-literal document.
pub(super) fn literal_tags(theme_color: &str) -> String {
    format!(
        "<link rel='manifest' href='/static/manifest.webmanifest'><meta name='theme-color' content='{theme_color}'><link rel='apple-touch-icon' href='/static/icons/apple-touch-icon.png'><script src='/static/pwa.js' defer></script>"
    )
}

struct Scan<'a> {
    source: &'a str,
    tags: &'a str,
    edits: Vec<(Range<usize>, String)>,
    wired: usize,
    present: usize,
    /// Rewritten `render_page_with_lang` calls, other uses of the name and
    /// where top-level `use` items import it.
    rewritten_calls: usize,
    other_lang_uses: usize,
    use_idents: Vec<Range<usize>>,
    imports_head_function: bool,
}

pub(super) fn wire(source: &str, theme_color: &str) -> Result<Wiring, Box<dyn Error>> {
    syn::parse_file(source)?;
    let tokens = source.parse::<TokenStream>()?;
    let tags = literal_tags(theme_color);
    let mut scan = Scan {
        source,
        tags: &tags,
        edits: Vec::new(),
        wired: 0,
        present: 0,
        rewritten_calls: 0,
        other_lang_uses: 0,
        use_idents: Vec::new(),
        imports_head_function: false,
    };
    let trees = tokens.into_iter().collect::<Vec<_>>();
    scan.walk(&trees, false, true)?;
    // The import follows the calls when they were its only users.
    if scan.rewritten_calls > 0 && scan.other_lang_uses == 0 && !scan.imports_head_function {
        for range in std::mem::take(&mut scan.use_idents) {
            scan.edits.push((range, "render_page_with_head".to_owned()));
        }
    }
    let mut edits = scan.edits;
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut updated = source.to_owned();
    for (range, text) in edits {
        if updated.get(range.clone()).is_none() {
            return Err(io::Error::other("invalid layout source range").into());
        }
        updated.replace_range(range, &text);
    }
    syn::parse_file(&updated)?;
    Ok(Wiring {
        source: updated,
        wired: scan.wired,
        present: scan.present,
    })
}

fn is_ident(tree: Option<&TokenTree>, name: &str) -> bool {
    matches!(tree, Some(TokenTree::Ident(ident)) if ident == name)
}

fn is_punct(tree: Option<&TokenTree>, character: char) -> bool {
    matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == character)
}

fn compact(stream: &TokenStream) -> String {
    stream.to_string().split_whitespace().collect()
}

impl Scan<'_> {
    /// Walks one token level. `top` marks the file level, where `use` items
    /// are collected for the import rename instead of being walked.
    fn walk(
        &mut self,
        trees: &[TokenTree],
        in_html: bool,
        top: bool,
    ) -> Result<(), Box<dyn Error>> {
        let mut in_use = false;
        let mut open_head = 0;
        for (index, tree) in trees.iter().enumerate() {
            let before = |distance: usize| index.checked_sub(distance).and_then(|at| trees.get(at));
            if top && is_ident(Some(tree), "use") {
                in_use = true;
            }
            if in_use {
                in_use = !is_punct(Some(tree), ';');
                self.collect_use(tree);
                continue;
            }
            match tree {
                TokenTree::Group(group) => {
                    let html_body = is_punct(before(1), '!') && is_ident(before(2), "html");
                    let inner = group.stream().into_iter().collect::<Vec<_>>();
                    self.walk(&inner, in_html || html_body, false)?;
                }
                TokenTree::Literal(literal) => self.literal(literal),
                TokenTree::Ident(ident) if ident == "render_page_with_lang" => {
                    if is_ident(before(1), "fn") || is_punct(before(1), '.') {
                        self.other_lang_uses += 1;
                    } else {
                        self.layout_call(tree, trees.get(index + 1));
                    }
                }
                TokenTree::Ident(ident) if ident == "render_page_with_head" => {
                    if matches!(trees.get(index + 1), Some(TokenTree::Group(arguments)) if compact(&arguments.stream()).contains("pwa::head()"))
                    {
                        self.present += 1;
                    }
                }
                TokenTree::Ident(ident) if in_html && ident == "head" => {
                    if is_punct(before(1), '<') {
                        open_head = index;
                    } else if let Some(close) = before(2)
                        && is_punct(before(1), '/')
                        && is_punct(Some(close), '<')
                        && is_punct(trees.get(index + 1), '>')
                    {
                        if links_manifest(trees.get(open_head..index).unwrap_or_default()) {
                            self.present += 1;
                        } else {
                            let start = close.span().byte_range().start;
                            self.edits.push((start..start, HEAD_CALL.to_owned()));
                            self.wired += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_use(&mut self, tree: &TokenTree) {
        match tree {
            TokenTree::Ident(ident) if ident == "render_page_with_lang" => {
                self.use_idents.push(ident.span().byte_range());
            }
            TokenTree::Ident(ident) if ident == "render_page_with_head" => {
                self.imports_head_function = true;
            }
            TokenTree::Group(group) => {
                for inner in group.stream() {
                    self.collect_use(&inner);
                }
            }
            _ => {}
        }
    }

    fn literal(&mut self, literal: &proc_macro2::Literal) {
        let repr = literal.to_string();
        if !(repr.starts_with('"') || repr.starts_with("r\"") || repr.starts_with("r#")) {
            return;
        }
        let range = literal.span().byte_range();
        // A doc comment is a `#[doc = "..."]` literal spanning the comment
        // text; documentation is never a layout.
        if self.source.get(range.clone()) != Some(repr.as_str()) {
            return;
        }
        // ASCII lower-casing keeps every byte offset.
        let lower = repr.to_ascii_lowercase();
        for (close, _) in lower.match_indices("</head>") {
            let open = lower
                .get(..close)
                .and_then(|before| before.rfind("<head"))
                .unwrap_or(0);
            if lower
                .get(open..close)
                .unwrap_or_default()
                .contains("manifest")
            {
                self.present += 1;
            } else {
                let at = range.start + close;
                self.edits.push((at..at, self.tags.to_owned()));
                self.wired += 1;
            }
        }
    }

    /// `render_page_with_lang(a, b, c, d)` becomes
    /// `render_page_with_head(a, b, c, crate::pwa::head(), d)`; any other
    /// argument list is counted as another use.
    fn layout_call(&mut self, name: &TokenTree, arguments: Option<&TokenTree>) {
        let Some(TokenTree::Group(arguments)) = arguments else {
            self.other_lang_uses += 1;
            return;
        };
        let inner = arguments.stream().into_iter().collect::<Vec<_>>();
        let commas = inner
            .iter()
            .enumerate()
            .filter(|(_, tree)| is_punct(Some(tree), ','))
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        let trailing = commas.len() == 4 && commas.last() == Some(&(inner.len() - 1));
        let valid =
            arguments.delimiter() == Delimiter::Parenthesis && (commas.len() == 3 || trailing);
        let Some(content) = commas
            .get(2)
            .and_then(|comma| inner.get(comma + 1))
            .filter(|_| valid)
        else {
            self.other_lang_uses += 1;
            return;
        };
        self.rewritten_calls += 1;
        self.wired += 1;
        let call = name.span().byte_range().start..arguments.span().byte_range().end;
        if let Some(single_line) = self.single_line_call(&call, &inner, &commas) {
            self.edits.push((call, single_line));
            return;
        }
        self.edits
            .push((name.span().byte_range(), "render_page_with_head".to_owned()));
        // In a call with one argument per line the new one gets its own line.
        let start = content.span().byte_range().start;
        let line = self.line_before(start);
        let insertion = if line.trim().is_empty() {
            format!("crate::pwa::head(),\n{line}")
        } else {
            "crate::pwa::head(), ".to_owned()
        };
        self.edits.push((start..start, insertion));
    }

    /// The text of the line that holds `offset`, up to `offset`.
    fn line_before(&self, offset: usize) -> &str {
        self.source
            .get(..offset)
            .and_then(|before| before.rsplit('\n').next())
            .unwrap_or_default()
    }

    /// A call written on one line that starts its line, rewritten the way
    /// rustfmt lays it out: on one line while the arguments fit its 60-column
    /// call width and the line its 100 columns, else one argument per line.
    fn single_line_call(
        &self,
        call: &Range<usize>,
        inner: &[TokenTree],
        commas: &[usize],
    ) -> Option<String> {
        let text = self.source.get(call.clone())?;
        let prefix = self.line_before(call.start);
        let indent = prefix.get(..prefix.len() - prefix.trim_start().len())?;
        // Only a module path (`rullst::htmx::`) may precede the name.
        let qualifier = prefix.trim_start();
        let starts_line = qualifier.is_empty()
            || (qualifier.ends_with("::")
                && qualifier.split("::").all(|segment| {
                    segment
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_')
                }));
        if text.contains('\n') || !starts_line {
            return None;
        }
        let rest = self
            .source
            .get(call.end..)
            .and_then(|after| after.split('\n').next())
            .unwrap_or_default();
        // Each argument spans the tokens between two top-level commas.
        let starts = std::iter::once(0).chain(commas.iter().map(|comma| comma + 1));
        let ends = commas.iter().copied().chain(std::iter::once(inner.len()));
        let mut arguments = starts
            .zip(ends)
            .filter(|(start, end)| start < end)
            .map(|(start, end)| {
                let first = inner.get(start)?.span().byte_range().start;
                let last = inner.get(end - 1)?.span().byte_range().end;
                self.source.get(first..last).map(str::to_owned)
            })
            .collect::<Option<Vec<_>>>()?;
        if arguments.len() != 4 {
            return None;
        }
        arguments.insert(3, "crate::pwa::head()".to_owned());
        let joined = arguments.join(", ");
        let line = format!("{prefix}render_page_with_head({joined}){rest}");
        if joined.chars().count() <= 60 && line.chars().count() <= 100 {
            return Some(format!("render_page_with_head({joined})"));
        }
        let lines = arguments
            .iter()
            .map(|argument| format!("{indent}    {argument},\n"))
            .collect::<String>();
        Some(format!("render_page_with_head(\n{lines}{indent})"))
    }
}

/// Whether the tokens of an `html!` head already link a manifest, through
/// `crate::pwa::head()` or an own `<link rel="manifest">`.
fn links_manifest(head: &[TokenTree]) -> bool {
    head.iter().enumerate().any(|(index, tree)| match tree {
        TokenTree::Group(group) => compact(&group.stream()).contains("pwa::head()"),
        TokenTree::Ident(ident) if ident == "rel" => {
            is_punct(head.get(index + 1), '=')
                && matches!(head.get(index + 2), Some(TokenTree::Literal(value)) if value.to_string() == "\"manifest\"")
        }
        _ => false,
    })
}

#[cfg(test)]
#[path = "head_tests.rs"]
mod tests;
