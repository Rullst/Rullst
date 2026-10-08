//! Structural injection signatures for the Core WAF.
//!
//! The text is percent-decoded and lowercased by the caller. Every check runs
//! on that text with whitespace runs collapsed to one space, and again with
//! each closed SQL comment (`/* … */`) replaced by a space. A keyword alone
//! never matches: SQL keywords need injection syntax around them and a command
//! name needs a preceding shell metacharacter, so "please select an option" or
//! "curl the API" pass while `1 union select …` and `; curl http://x` do not.
//! Every check scans the text once with bounded look-ahead.

/// Cross-site scripting signatures, matched as substrings.
const XSS_SIGNATURES: &[&str] = &[
    "<script",
    "javascript:",
    "onload=",
    "onerror=",
    "document.cookie",
];

/// Functions, objects and clauses that only SQL injection probes use.
const SQL_PROBES: &[&str] = &[
    "sleep(",
    "benchmark(",
    "waitfor delay",
    "pg_sleep(",
    "information_schema",
    "@@version",
    "xp_cmdshell",
    "load_file(",
    "into outfile",
    "into dumpfile",
];

/// Commands an injected shell fragment typically runs.
const SHELL_COMMANDS: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "cat",
    "ls",
    "id",
    "whoami",
    "curl",
    "wget",
    "nc",
    "ncat",
    "ping",
    "rm",
    "python",
    "perl",
    "php",
    "powershell",
    "cmd",
];

/// Bytes that may end a command name: the end of a shell word.
const COMMAND_END: &[u8] = b" ;|&`)<>$'\"";

/// Objects a chained `drop`, `create` or `alter` statement names.
const SCHEMA_OBJECTS: &[&str] = &[
    "table",
    "database",
    "schema",
    "view",
    "index",
    "user",
    "procedure",
    "function",
    "trigger",
];

/// How many bytes after a quote-breaking `or`/`and` may hold a comparison.
const OPERATOR_WINDOW: usize = 32;

/// Whether the decoded, lowercased `text` carries an injection signature.
pub(super) fn is_injection(text: &str) -> bool {
    let collapsed = collapse(text.as_bytes(), false);
    if matches_signature(&collapsed) {
        return true;
    }
    let stripped = collapse(text.as_bytes(), true);
    stripped != collapsed && matches_signature(&stripped)
}

fn matches_signature(text: &[u8]) -> bool {
    XSS_SIGNATURES
        .iter()
        .chain(SQL_PROBES)
        .any(|signature| contains(text, signature.as_bytes()))
        || (0..text.len()).any(|at| structural_match(text, at))
}

/// Checks the structures that start at `at`.
fn structural_match(text: &[u8], at: usize) -> bool {
    match text[at] {
        b'\'' | b'"' => quote_breaks_out(text, at),
        b';' => chains_statement(text, at + 1) || runs_command(text, at + 1),
        b'|' | b'`' => runs_command(text, at + 1),
        b'&' if text.get(at + 1) == Some(&b'&') => runs_command(text, at + 2),
        b'$' if text.get(at + 1) == Some(&b'(') => runs_command(text, at + 2),
        b'u' => (at == 0 || !is_word_byte(text[at - 1])) && unions_select(&text[at..]),
        _ => false,
    }
}

/// Collapses ASCII whitespace runs to one space and, when `strip_comments` is
/// set, replaces each closed `/* … */` comment with a space.
fn collapse(text: &[u8], strip_comments: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut unclosed_comment = false;
    let mut at = 0;
    while at < text.len() {
        if strip_comments && !unclosed_comment && text[at..].starts_with(b"/*") {
            match find(&text[at + 2..], b"*/") {
                Some(end) => {
                    push_space(&mut out);
                    at += end + 4;
                    continue;
                }
                // No later comment can close either: stop searching.
                None => unclosed_comment = true,
            }
        }
        if text[at].is_ascii_whitespace() {
            push_space(&mut out);
        } else {
            out.push(text[at]);
        }
        at += 1;
    }
    out
}

fn push_space(out: &mut Vec<u8>) {
    if out.last() != Some(&b' ') {
        out.push(b' ');
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

/// Identifier bytes; non-ASCII bytes count so accented words stay whole.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

fn skip_spaces(text: &[u8], mut at: usize) -> usize {
    while text.get(at) == Some(&b' ') {
        at += 1;
    }
    at
}

/// The offset after `word` when `text[at..]` starts with it as a whole word.
fn word_at(text: &[u8], at: usize, word: &str) -> Option<usize> {
    let end = at + word.len();
    let rest = text.get(at..)?;
    (rest.starts_with(word.as_bytes()) && !text.get(end).is_some_and(|b| is_word_byte(*b)))
        .then_some(end)
}

/// The offset after the next word when, after optional spaces, it is one of `words`.
fn then_any_word(text: &[u8], at: usize, words: &[&str]) -> Option<usize> {
    let at = skip_spaces(text, at);
    words.iter().find_map(|word| word_at(text, at, word))
}

/// The offset after an identifier (letters, digits, `_` and `.`) at `at`.
fn identifier_end(text: &[u8], at: usize) -> Option<usize> {
    let length = text
        .get(at..)?
        .iter()
        .take_while(|byte| is_word_byte(**byte) || **byte == b'.')
        .count();
    (length > 0).then_some(at + length)
}

/// `' or '1'='1`, `" or ""="`, `admin'--`, `admin'#`: a quote followed by a
/// boolean operator and a comparison, or by a SQL comment.
fn quote_breaks_out(text: &[u8], quote: usize) -> bool {
    let mut at = quote + 1;
    while matches!(text.get(at), Some(b' ' | b')')) {
        at += 1;
    }
    let rest = &text[at.min(text.len())..];
    if rest.starts_with(b"--") || rest.starts_with(b"/*") {
        return true;
    }
    if rest.starts_with(b"#") {
        // `#` opens a MySQL comment; `"#fff"` or `'#1'` open a value instead.
        return quote > 0 && is_word_byte(text[quote - 1]);
    }
    let operator = then_any_word(text, at, &["or", "and", "xor"])
        .or_else(|| (rest.starts_with(b"||") || rest.starts_with(b"&&")).then_some(at + 2));
    operator.is_some_and(|end| {
        let window = &text[end..text.len().min(end + OPERATOR_WINDOW)];
        window
            .iter()
            .any(|byte| matches!(byte, b'=' | b'<' | b'>' | b'#'))
            || contains(window, b"--")
            || contains(window, b"like")
    })
}

/// `; drop table`, `; delete from`, `; select * …`: a second statement after
/// a `;`, recognised by its verb and the clause that must follow it.
fn chains_statement(text: &[u8], at: usize) -> bool {
    let at = skip_spaces(text, at);
    let verb = |word| word_at(text, at, word);
    if let Some(end) = verb("drop")
        .or_else(|| verb("create"))
        .or_else(|| verb("alter"))
    {
        return then_any_word(text, end, SCHEMA_OBJECTS).is_some();
    }
    if let Some(end) = verb("truncate") {
        return then_any_word(text, end, &["table"]).is_some();
    }
    if let Some(end) = verb("delete") {
        return then_any_word(text, end, &["from"]).is_some();
    }
    if let Some(end) = verb("insert") {
        return then_any_word(text, end, &["into"]).is_some();
    }
    if let Some(end) = verb("update") {
        return identifier_end(text, skip_spaces(text, end))
            .and_then(|table| then_any_word(text, table, &["set"]))
            .is_some();
    }
    if let Some(end) = verb("select") {
        return selects_columns(text, skip_spaces(text, end));
    }
    if let Some(end) = verb("declare") {
        return text.get(skip_spaces(text, end)) == Some(&b'@');
    }
    verb("shutdown").is_some() || verb("exec").is_some()
}

/// `select *`, `select 1`, `select @@…` or `select a, b from`.
fn selects_columns(text: &[u8], at: usize) -> bool {
    match text.get(at) {
        Some(b'*' | b'@' | b'(' | b'\'' | b'"') => return true,
        Some(byte) if byte.is_ascii_digit() => return true,
        _ => {}
    }
    let Some(mut end) = identifier_end(text, at) else {
        return false;
    };
    loop {
        let next = skip_spaces(text, end);
        if text.get(next) != Some(&b',') {
            break;
        }
        match identifier_end(text, skip_spaces(text, next + 1)) {
            Some(column) => end = column,
            None => return false,
        }
    }
    then_any_word(text, end, &["from"]).is_some()
}

/// `union select`, `union all select`, `union (select`.
fn unions_select(text: &[u8]) -> bool {
    let Some(mut at) = word_at(text, 0, "union") else {
        return false;
    };
    let skip_open = |mut at: usize| {
        while matches!(text.get(at), Some(b' ' | b'(')) {
            at += 1;
        }
        at
    };
    at = skip_open(at);
    if let Some(end) = word_at(text, at, "all").or_else(|| word_at(text, at, "distinct")) {
        at = skip_open(end);
    }
    word_at(text, at, "select").is_some()
}

/// A command name, optionally behind a path such as `/bin/`, starting at
/// `at` after a shell metacharacter.
fn runs_command(text: &[u8], at: usize) -> bool {
    let start = skip_spaces(text, at);
    let Some(token) = text.get(start..) else {
        return false;
    };
    let token_length = token
        .iter()
        .take_while(|byte| is_word_byte(**byte) || matches!(byte, b'/' | b'.' | b'-'))
        .count();
    let name = token[..token_length]
        .iter()
        .rposition(|byte| *byte == b'/')
        .map_or(start, |slash| start + slash + 1);
    SHELL_COMMANDS.iter().any(|command| {
        if !text[name..].starts_with(command.as_bytes()) {
            return false;
        }
        let mut end = name + command.len();
        while text.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        text.get(end).is_none_or(|byte| COMMAND_END.contains(byte))
    })
}
