//! Finding the project's page layout: the one place in `src/` where an
//! `html!` invocation closes `<head>`. Only that shape is edited; any other
//! project gets the exact line to paste.

use proc_macro2::{TokenStream, TokenTree};
use std::path::Path;
use syn::visit::Visit;

/// Largest source file inspected.
const SOURCE_LIMIT: u64 = 1024 * 1024;

/// The layout: `path` (relative to the project, with `/`) closes `<head>`
/// at 1-based `line` inside `html!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layout {
    pub path: String,
    pub text: String,
    pub line: usize,
}

/// What the search found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Search {
    Found(Layout),
    /// No `html!` closes `<head>`: pages use `render_page*` or hand-written
    /// strings.
    NotFound,
    /// `</head>` closes in several places (files listed).
    Ambiguous(Vec<String>),
}

/// Lines of `</head>` (the `<` token) inside `html!` invocations of `source`.
pub(crate) fn head_closings(source: &str) -> Vec<usize> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut visitor = HtmlMacros { lines: Vec::new() };
    visitor.visit_file(&file);
    visitor.lines
}

struct HtmlMacros {
    lines: Vec<usize>,
}

impl<'ast> Visit<'ast> for HtmlMacros {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "html")
        {
            closing_heads(mac.tokens.clone(), &mut self.lines);
        }
        syn::visit::visit_macro(self, mac);
    }
}

/// Collects the line of every `<` `/` `head` `>` sequence, in nested groups too.
fn closing_heads(tokens: TokenStream, lines: &mut Vec<usize>) {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            closing_heads(group.stream(), lines);
            continue;
        }
        let window = &tokens[index..tokens.len().min(index + 4)];
        if let [
            TokenTree::Punct(open),
            TokenTree::Punct(slash),
            TokenTree::Ident(name),
            TokenTree::Punct(close),
        ] = window
            && open.as_char() == '<'
            && slash.as_char() == '/'
            && name == "head"
            && close.as_char() == '>'
        {
            lines.push(open.span().start().line);
        }
    }
}

/// The first source file in `src/` (relative path) containing one of
/// `needles`, which shows the wiring is already there.
pub(crate) fn mentioning(root: &Path, needles: &[&str]) -> Option<String> {
    let sources = crate::generators::source_walk::rust_sources(&root.join("src"));
    sources.files.into_iter().find_map(|file| {
        let text = read_source(&file)?;
        needles
            .iter()
            .any(|needle| text.contains(needle))
            .then(|| relative(root, &file))
    })
}

fn read_source(file: &Path) -> Option<String> {
    let small = std::fs::metadata(file).is_ok_and(|metadata| metadata.len() <= SOURCE_LIMIT);
    small.then(|| std::fs::read_to_string(file).ok()).flatten()
}

fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Looks for the layout in the project's `src/`.
pub(crate) fn find(root: &Path) -> Search {
    let sources = crate::generators::source_walk::rust_sources(&root.join("src"));
    let mut found = Vec::new();
    for file in sources.files {
        let Some(text) = read_source(&file) else {
            continue;
        };
        if !text.contains("</head>") {
            continue;
        }
        let path = relative(root, &file);
        for line in head_closings(&text) {
            found.push(Layout {
                path: path.clone(),
                text: text.clone(),
                line,
            });
        }
    }
    match found.len() {
        0 => Search::NotFound,
        1 => found.pop().map_or(Search::NotFound, Search::Found),
        _ => {
            let mut files: Vec<String> = found.into_iter().map(|layout| layout.path).collect();
            files.dedup();
            Search::Ambiguous(files)
        }
    }
}

/// `layout.text` with `lines` (already indented by this function to match
/// `</head>`'s indentation plus one level) inserted before the `</head>` line.
/// `None` when `</head>` is not alone on its line, which this edit does not
/// rewrite.
pub(crate) fn insert_before_head(layout: &Layout, lines: &[&str]) -> Option<String> {
    let newline = if layout.text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = String::with_capacity(layout.text.len() + 256);
    let mut inserted = false;
    for (index, line) in layout.text.split_inclusive('\n').enumerate() {
        if index + 1 == layout.line {
            let trimmed = line.trim();
            if !trimmed.starts_with("</head>") {
                return None;
            }
            let indent: String = line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();
            for inserted_line in lines {
                out.push_str(&format!("{indent}    {inserted_line}{newline}"));
            }
            inserted = true;
        }
        out.push_str(line);
    }
    inserted.then_some(out)
}
