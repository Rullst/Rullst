//! `V13-CSRF-TOKEN-COMPARED` in an application's JavaScript, TypeScript and
//! Python files, such as browser (Playwright) and smoke tests. Since
//! 13.0.0-alpha.6 the `_token` a page embeds is masked for each response, so
//! a test that checks the form's token against the `rullst_csrf` cookie fails
//! although the form works, and `cargo test` does not run such scripts.
//!
//! Files are read line by line with a small lexer that drops comments and
//! keeps string literals apart from code, in files that mention CSRF. A line
//! is reported when it expects the cookie and the token to be equal: an
//! `==`/`===` between them (a `!=`/`!==` in the condition of an `if` that
//! fails the test), an equality assertion (`assert.equal`,
//! `expect(..).toBe(..)`, `assertEqual`), or a check that a page contains the
//! cookie's value (`.includes(..)`, `toContain`, Python's `in`). Names
//! assigned from a cookie read (`const csrf = cookies.find(..)`) count as the
//! cookie. Sending the cookie's value as `_token` or `X-CSRF-Token`, which the
//! middleware still accepts, compares nothing and is not reported.

use super::read_small;
use super::script_tokens::{
    Lexer, Token, arguments, group_end, operand_end, operand_start, punct, receiver_start, word,
};
use std::collections::BTreeSet;
use std::path::Path;
use walkdir::WalkDir;

/// Directory entries visited at most.
const MAX_ENTRIES: usize = 5_000;
/// Directories below the workspace root that are read.
const MAX_DEPTH: usize = 5;
/// Extensions of the scripts read; `py` selects Python's syntax.
const EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx", "py"];
/// Directories never read: build output, dependencies and version control.
const SKIPPED: &[&str] = &[
    ".git",
    ".rullst",
    ".venv",
    "__pycache__",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "venv",
];
/// Calls that assert that their first two arguments are equal.
const EQUAL_CALLS: &[&str] = &[
    "assertEqual",
    "assertEquals",
    "deepEqual",
    "deepStrictEqual",
    "equal",
    "strictEqual",
];
/// Matchers of an `expect(..)` chain that assert equality.
const EQUAL_MATCHERS: &[&str] = &["toBe", "toEqual", "toHaveValue", "toStrictEqual"];

/// What the names and string literals of an operand mention.
#[derive(Debug, Default, Clone, Copy)]
struct Mentions {
    /// A name containing `cookie`, or a variable assigned from a cookie read.
    cookie: bool,
    /// A string literal containing `cookie` or `rullst_csrf=`, as in a
    /// `Set-Cookie` header.
    cookie_text: bool,
    /// A name containing `token` or `csrf`, or a string containing `token`.
    token: bool,
}

impl Mentions {
    fn reads_cookie(self) -> bool {
        self.cookie || self.cookie_text
    }

    /// One side reads the cookie and the other is a token that does not.
    fn compared_with(self, other: Self) -> bool {
        let pair = |cookie: Self, token: Self| {
            cookie.reads_cookie() && token.token && !token.reads_cookie()
        };
        pair(self, other) || pair(other, self)
    }

    /// `self` is the cookie's value looked for in `haystack`, which is not a
    /// cookie (a `Set-Cookie` header is one).
    fn found_in(self, haystack: Self) -> bool {
        self.cookie && !haystack.reads_cookie()
    }
}

/// What `tokens` mention; `cookies` are the variables assigned from a cookie.
fn mentions(tokens: &[Token], cookies: &BTreeSet<String>) -> Mentions {
    let mut mentions = Mentions::default();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Token::Word(name) => {
                let lower = name.to_ascii_lowercase();
                // A variable, not a property of the same name (`x.value`).
                let property = index.checked_sub(1).is_some_and(|before| {
                    punct(tokens.get(before), ".") || punct(tokens.get(before), "?.")
                });
                mentions.cookie |=
                    lower.contains("cookie") || (!property && cookies.contains(name));
                mentions.token |= lower.contains("token") || lower.contains("csrf");
            }
            Token::Text(text) => {
                let lower = text.to_ascii_lowercase();
                mentions.cookie_text |= lower.contains("cookie") || lower.contains("rullst_csrf=");
                mentions.token |= lower.contains("token");
            }
            _ => {}
        }
    }
    mentions
}

/// One line's tokens and the names assigned from a cookie read so far.
struct Line<'a> {
    tokens: &'a [Token],
    cookies: &'a BTreeSet<String>,
    python: bool,
    /// The token range of the condition of an `if` that starts the line.
    condition: Option<(usize, usize)>,
}

impl Line<'_> {
    fn mentions(&self, tokens: &[Token]) -> Mentions {
        mentions(tokens, self.cookies)
    }

    /// Whether the token at `index` is in the condition of an `if`, which
    /// fails the test when it holds.
    fn guarded(&self, index: usize) -> bool {
        self.condition
            .is_some_and(|(start, end)| (start..end).contains(&index))
    }

    /// Whether the line expects the cookie and the page's token to be equal.
    fn compares(&self) -> bool {
        (0..self.tokens.len()).any(|index| self.compares_at(index))
    }

    fn compares_at(&self, index: usize) -> bool {
        let tokens = self.tokens;
        let opens_call = matches!(tokens.get(index + 1), Some(Token::Open));
        match &tokens[index] {
            Token::Punct(operator) if matches!(operator.as_str(), "===" | "==" | "!==" | "!=") => {
                let equal = operator.starts_with('=');
                let left = &tokens[operand_start(tokens, index)..index];
                let right = &tokens[index + 1..operand_end(tokens, index + 1)];
                equal != self.guarded(index)
                    && self.mentions(left).compared_with(self.mentions(right))
            }
            Token::Word(name) if opens_call && EQUAL_CALLS.contains(&name.as_str()) => {
                matches!(arguments(tokens, index + 1).as_slice(), [first, second, ..]
                    if self.mentions(first).compared_with(self.mentions(second)))
            }
            Token::Word(name) if opens_call && name == "assertIn" => {
                matches!(arguments(tokens, index + 1).as_slice(), [needle, haystack, ..]
                    if self.mentions(needle).found_in(self.mentions(haystack)))
            }
            Token::Word(name) if opens_call && name == "expect" => self.expectation(index + 1),
            Token::Word(name)
                if opens_call
                    && name == "includes"
                    && punct(index.checked_sub(1).and_then(|at| tokens.get(at)), ".") =>
            {
                let start = receiver_start(tokens, index - 1);
                let negated = punct(start.checked_sub(1).and_then(|at| tokens.get(at)), "!");
                let needle = &tokens[index + 2..group_end(tokens, index + 1)];
                negated == self.guarded(index)
                    && self
                        .mentions(needle)
                        .found_in(self.mentions(&tokens[start..index - 1]))
            }
            Token::Word(name) if self.python && name == "in" => self.python_in(index),
            _ => false,
        }
    }

    /// An `expect(subject)` chain whose group opens at `open`: `.toBe(..)`
    /// and the other equality matchers, and `.toContain(..)`, unless `.not`
    /// negates them.
    fn expectation(&self, open: usize) -> bool {
        let tokens = self.tokens;
        let close = group_end(tokens, open);
        let subject = self.mentions(&tokens[(open + 1).min(close)..close]);
        let mut negated = false;
        let mut index = close + 1;
        while punct(tokens.get(index), ".") {
            let Some(Token::Word(link)) = tokens.get(index + 1) else {
                return false;
            };
            if !matches!(tokens.get(index + 2), Some(Token::Open)) {
                negated |= link == "not";
                index += 2;
                continue;
            }
            let expected = &tokens[index + 3..group_end(tokens, index + 2)];
            let expected = self.mentions(expected);
            return !negated
                && if link == "toContain" {
                    expected.found_in(subject)
                } else {
                    EQUAL_MATCHERS.contains(&link.as_str()) && subject.compared_with(expected)
                };
        }
        false
    }

    /// Python's `needle in haystack` (`not in` negates it), except the
    /// `for name in ...` of a loop or comprehension.
    fn python_in(&self, index: usize) -> bool {
        let tokens = self.tokens;
        let negated = word(index.checked_sub(1).and_then(|at| tokens.get(at)), "not");
        let end = if negated { index - 1 } else { index };
        let start = operand_start(tokens, end);
        if word(start.checked_sub(1).and_then(|at| tokens.get(at)), "for") {
            return false;
        }
        let haystack = &tokens[index + 1..operand_end(tokens, index + 1)];
        negated == self.guarded(index)
            && self
                .mentions(&tokens[start..end])
                .found_in(self.mentions(haystack))
    }
}

/// The token range of the condition of an `if` (`elif`, `} else if`) that
/// starts the line: its parenthesized group, or up to Python's `:`.
fn condition(tokens: &[Token]) -> Option<(usize, usize)> {
    let start = tokens
        .iter()
        .position(|token| !(matches!(token, Token::Close) || word(Some(token), "else")))?;
    if !(word(tokens.get(start), "if") || word(tokens.get(start), "elif")) {
        return None;
    }
    if matches!(tokens.get(start + 1), Some(Token::Open)) {
        return Some((start + 2, group_end(tokens, start + 1)));
    }
    let end = (start + 1..tokens.len())
        .find(|index| punct(tokens.get(*index), ":"))
        .unwrap_or(tokens.len());
    Some((start + 1, end))
}

/// Records `name = value` (also after `const`, `let` or `var`, and with a
/// type annotation): `name` reads the cookie when `value` does.
fn bind(tokens: &[Token], cookies: &mut BTreeSet<String>) {
    let start = usize::from(
        matches!(tokens.first(), Some(Token::Word(keyword)) if matches!(keyword.as_str(), "const" | "let" | "var")),
    );
    let Some(Token::Word(name)) = tokens.get(start) else {
        return;
    };
    let equals = if punct(tokens.get(start + 1), "=") {
        start + 1
    } else if punct(tokens.get(start + 1), ":") {
        match (start + 2..tokens.len()).find(|index| punct(tokens.get(*index), "=")) {
            Some(index) => index,
            None => return,
        }
    } else {
        return;
    };
    let value = &tokens[equals + 1..];
    // Also a lookup by the cookie's name: `c.name == "rullst_csrf"`.
    let reads_cookie = mentions(value, cookies).reads_cookie()
        || value
            .iter()
            .any(|token| matches!(token, Token::Text(text) if text.contains("rullst_csrf")));
    if reads_cookie {
        cookies.insert(name.clone());
    } else {
        cookies.remove(name);
    }
}

/// The 1-based lines of a script that expect the CSRF cookie and the page's
/// token to be equal; `python` selects Python's syntax.
pub(in super::super) fn compared_lines(text: &str, python: bool) -> Vec<usize> {
    let mut lexer = Lexer::new(python);
    let mut cookies = BTreeSet::new();
    let mut found = Vec::new();
    for (index, text) in text.lines().enumerate() {
        let tokens = lexer.line(text);
        let line = Line {
            tokens: &tokens,
            cookies: &cookies,
            python,
            condition: condition(&tokens),
        };
        if line.compares() {
            found.push(index + 1);
        }
        bind(&tokens, &mut cookies);
    }
    found
}

/// The relative path and line of every comparison in the JavaScript,
/// TypeScript and Python files below `root` that mention CSRF.
pub(in super::super) fn comparisons(root: &Path) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| SKIPPED.contains(&name))
        })
        .filter_map(Result::ok)
        .take(MAX_ENTRIES)
    {
        let name = entry.file_name().to_string_lossy();
        let Some((stem, extension)) = name.rsplit_once('.') else {
            continue;
        };
        if !entry.file_type().is_file()
            || stem.is_empty()
            || stem.ends_with(".min")
            || !EXTENSIONS.contains(&extension)
        {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let Some(text) = read_small(entry.path()) else {
            continue;
        };
        if !text.to_ascii_lowercase().contains("csrf") {
            continue;
        }
        let relative = relative.to_string_lossy().replace('\\', "/");
        for line in compared_lines(&text, extension == "py") {
            found.push((relative.clone(), line));
        }
    }
    found
}
