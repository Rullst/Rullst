//! A small line lexer for the JavaScript, TypeScript and Python files that
//! [`super::csrf_scripts`] reads: names, string literals, operators and
//! brackets, with comments dropped and block comments, template literals and
//! triple-quoted strings carried across lines, and the operand and argument
//! boundaries the comparisons need. It reads code well enough to locate a
//! comparison on one line; it is not a parser.

/// Operators read as one token, longest first.
const OPERATORS: &[&str] = &[
    "===", "!==", "==", "!=", "<=", ">=", "&&", "||", "??", "?.", "=>",
];
/// Operators and punctuation that end an operand.
const BOUNDARY_PUNCTUATION: &[&str] = &[
    ",", ";", "=", "===", "!==", "==", "!=", "&&", "||", "??", "?", ":", "=>",
];
/// Keywords that end an operand.
pub(super) const CLAUSE_WORDS: &[&str] = &[
    "and", "assert", "const", "elif", "else", "for", "if", "in", "lambda", "let", "not", "or",
    "return", "throw", "var", "while", "yield",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    /// An identifier, keyword or number.
    Word(String),
    /// The contents of a string literal.
    Text(String),
    Punct(String),
    Open,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Code,
    BlockComment,
    /// Inside a string; only backtick and triple-quoted strings span lines.
    Quote {
        delimiter: char,
        triple: bool,
    },
}

/// Reads a file line by line; `python` selects Python's comments and strings.
pub(super) struct Lexer {
    python: bool,
    state: State,
}

impl Lexer {
    pub(super) fn new(python: bool) -> Self {
        Self {
            python,
            state: State::Code,
        }
    }

    /// The tokens of one line, carrying block comments and multi-line strings
    /// over to the next.
    pub(super) fn line(&mut self, line: &str) -> Vec<Token> {
        let chars: Vec<char> = line.chars().collect();
        let mut tokens = Vec::new();
        let mut index = 0;
        while index < chars.len() {
            match self.state {
                State::BlockComment => {
                    match (index..chars.len()).find(|at| chars[*at..].starts_with(&['*', '/'])) {
                        Some(end) => {
                            index = end + 2;
                            self.state = State::Code;
                        }
                        None => index = chars.len(),
                    }
                }
                State::Quote { delimiter, triple } => {
                    let (text, end, closed) = quoted(&chars, index, delimiter, triple);
                    tokens.push(Token::Text(text));
                    index = end;
                    if closed {
                        self.state = State::Code;
                    }
                }
                State::Code => index = self.code(&chars, index, &mut tokens),
            }
        }
        if let State::Quote { delimiter, triple } = self.state
            && !triple
            && delimiter != '`'
        {
            self.state = State::Code;
        }
        tokens
    }

    /// Reads one token of code at `index`; returns the index after it.
    fn code(&mut self, chars: &[char], index: usize, tokens: &mut Vec<Token>) -> usize {
        let character = chars[index];
        let next = chars.get(index + 1).copied();
        if character.is_whitespace() {
            return index + 1;
        }
        if (self.python && character == '#')
            || (!self.python && character == '/' && next == Some('/'))
        {
            return chars.len();
        }
        if !self.python && character == '/' && next == Some('*') {
            self.state = State::BlockComment;
            return index + 2;
        }
        if character == '"' || character == '\'' || (character == '`' && !self.python) {
            let triple = self.python && chars[index..].starts_with(&[character; 3]);
            self.state = State::Quote {
                delimiter: character,
                triple,
            };
            return index + if triple { 3 } else { 1 };
        }
        if is_word(character) {
            let end = (index..chars.len())
                .find(|at| !is_word(chars[*at]))
                .unwrap_or(chars.len());
            tokens.push(Token::Word(chars[index..end].iter().collect()));
            return end;
        }
        match character {
            '(' | '[' | '{' => tokens.push(Token::Open),
            ')' | ']' | '}' => tokens.push(Token::Close),
            _ => {
                let rest: String = chars[index..].iter().take(3).collect();
                let operator = OPERATORS
                    .iter()
                    .find(|operator| rest.starts_with(**operator))
                    .map_or_else(|| character.to_string(), |operator| (*operator).to_string());
                let length = operator.chars().count();
                tokens.push(Token::Punct(operator));
                return index + length;
            }
        }
        index + 1
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

/// The text of a string from `start` to its closing delimiter or the end of
/// the line, the index after it, and whether it closed.
fn quoted(chars: &[char], start: usize, delimiter: char, triple: bool) -> (String, usize, bool) {
    let mut text = String::new();
    let mut index = start;
    while index < chars.len() {
        let character = chars[index];
        if character == '\\' {
            text.extend(chars.get(index + 1));
            index += 2;
            continue;
        }
        if triple && chars[index..].starts_with(&[delimiter; 3]) {
            return (text, index + 3, true);
        }
        if !triple && character == delimiter {
            return (text, index + 1, true);
        }
        text.push(character);
        index += 1;
    }
    (text, chars.len(), false)
}

pub(super) fn punct(token: Option<&Token>, expected: &str) -> bool {
    matches!(token, Some(Token::Punct(found)) if found == expected)
}

pub(super) fn word(token: Option<&Token>, expected: &str) -> bool {
    matches!(token, Some(Token::Word(found)) if found == expected)
}

/// The index of the `Close` that ends the group opened at `open`.
pub(super) fn group_end(tokens: &[Token], open: usize) -> usize {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::Open => depth += 1,
            Token::Close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return index;
                }
            }
            _ => {}
        }
    }
    tokens.len()
}

/// The arguments of the group opened at `open`, split at its own commas.
pub(super) fn arguments(tokens: &[Token], open: usize) -> Vec<&[Token]> {
    let end = group_end(tokens, open);
    let inner = &tokens[(open + 1).min(end)..end];
    let mut found = Vec::new();
    let (mut depth, mut first) = (0usize, 0);
    for (index, token) in inner.iter().enumerate() {
        match token {
            Token::Open => depth += 1,
            Token::Close => depth = depth.saturating_sub(1),
            Token::Punct(comma) if comma == "," && depth == 0 => {
                found.push(&inner[first..index]);
                first = index + 1;
            }
            _ => {}
        }
    }
    found.push(&inner[first..]);
    found
}

/// Whether the token ends an operand outside brackets.
fn boundary(token: &Token) -> bool {
    match token {
        Token::Punct(found) => BOUNDARY_PUNCTUATION.contains(&found.as_str()),
        Token::Word(found) => CLAUSE_WORDS.contains(&found.as_str()),
        _ => false,
    }
}

/// The first index of the operand that ends before `end`.
pub(super) fn operand_start(tokens: &[Token], end: usize) -> usize {
    let mut depth = 0usize;
    let mut index = end;
    while index > 0 {
        match &tokens[index - 1] {
            Token::Close => depth += 1,
            Token::Open if depth == 0 => break,
            Token::Open => depth -= 1,
            token if depth == 0 && boundary(token) => break,
            _ => {}
        }
        index -= 1;
    }
    index
}

/// The index after the operand that starts at `start`.
pub(super) fn operand_end(tokens: &[Token], start: usize) -> usize {
    let mut depth = 0usize;
    let mut index = start;
    while index < tokens.len() {
        match &tokens[index] {
            Token::Open => depth += 1,
            Token::Close if depth == 0 => break,
            Token::Close => depth -= 1,
            token if depth == 0 && boundary(token) => break,
            _ => {}
        }
        index += 1;
    }
    index
}

/// The first index of the receiver chain of a method called after the `.`
/// at `dot`: names, `.`, `?.`, string literals and bracketed groups.
pub(super) fn receiver_start(tokens: &[Token], dot: usize) -> usize {
    let mut depth = 0usize;
    let mut index = dot;
    while index > 0 {
        let link = match &tokens[index - 1] {
            Token::Close => {
                depth += 1;
                true
            }
            Token::Open if depth > 0 => {
                depth -= 1;
                true
            }
            _ if depth > 0 => true,
            Token::Word(found) => !CLAUSE_WORDS.contains(&found.as_str()),
            Token::Text(_) => true,
            Token::Punct(found) => found == "." || found == "?.",
            Token::Open => false,
        };
        if !link {
            break;
        }
        index -= 1;
    }
    index
}
