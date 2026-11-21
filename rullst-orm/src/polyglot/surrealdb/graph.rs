use async_trait::async_trait;
use serde_json::Value;

use super::PolyglotError;

const MAX_GRAPH_QUERY_BYTES: usize = 64 * 1024;
const MAX_GRAPH_ROWS: u32 = 1_000;

const NOT_MATCH: &str = "read-only graph queries must start with MATCH";
const OUTSIDE_GRAMMAR: &str = "only MATCH paths, WHERE comparisons, RETURN items and ORDER BY are accepted; the adapter adds LIMIT";
const UNKNOWN_CHARACTER: &str = "character outside the read-only graph grammar";
const BAD_STRING: &str = "string literals must be closed and contain no backslash";
const BAD_NUMBER: &str = "numbers must be unsigned integers or decimals";

/// A validated read-only ISO GQL query with a framework-controlled limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphQuery {
    query: String,
    pub(super) limit: u32,
}

impl GraphQuery {
    /// Accepts one query of the read-only grammar below and appends `LIMIT`.
    ///
    /// The query is `MATCH`, one or more comma-separated paths, an optional
    /// `WHERE`, `RETURN` and an optional `ORDER BY`:
    ///
    /// - a path alternates nodes `(variable:label)` and edges
    ///   `-[variable:label]->`, `<-[variable:label]-` or `-[variable:label]-`;
    ///   the variable and the label are optional;
    /// - `WHERE` compares a variable or property (`n.age`) with `=`, `<>`, `<`,
    ///   `<=`, `>` or `>=` to another one, an unsigned number or a quoted
    ///   string without backslashes, or tests it with `IS [NOT] NULL`; tests
    ///   are joined with `AND`, `OR` and `NOT`, without parentheses;
    /// - `RETURN` lists variables or properties, each with an optional
    ///   `AS alias`;
    /// - `ORDER BY` lists variables, properties or aliases, each with an
    ///   optional `ASC` or `DESC`.
    ///
    /// These keywords and `LIMIT` are case-insensitive and cannot be names;
    /// tokens are separated by ASCII whitespace only. Anything else is refused
    /// with [`PolyglotError::InvalidIdentifier`], including every write or
    /// catalog clause, procedure and function calls, comments, parameters,
    /// semicolons and a caller-provided `LIMIT`. The grammar is an allowlist:
    /// a clause that a later SurrealDB release adds stays refused.
    pub fn read_only(query: impl Into<String>, limit: u32) -> Result<Self, PolyglotError> {
        let query = query.into();
        let trimmed = query.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_GRAPH_QUERY_BYTES {
            return Err(invalid_graph_query(
                "query must contain 1-65536 UTF-8 bytes",
            ));
        }
        if limit == 0 || limit > MAX_GRAPH_ROWS {
            return Err(invalid_graph_query("limit must be between 1 and 1000"));
        }
        if trimmed.contains(';') {
            return Err(invalid_graph_query(
                "multiple statements and semicolons are not accepted",
            ));
        }
        validate_read_grammar(trimmed).map_err(invalid_graph_query)?;
        Ok(Self {
            query: trimmed.to_owned(),
            limit,
        })
    }

    pub(super) fn bounded_query(&self) -> String {
        format!("{} LIMIT {}", self.query, self.limit)
    }
}

/// Read-only graph query boundary for SurrealDB's ISO GQL endpoint.
#[async_trait]
pub trait GraphRepository: Send + Sync {
    /// Runs a validated, bounded graph query and returns its JSON rows.
    async fn query_graph(&self, query: &GraphQuery) -> Result<Vec<Value>, PolyglotError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Keyword {
    Match,
    Where,
    Return,
    As,
    Order,
    By,
    Asc,
    Desc,
    And,
    Or,
    Not,
    Is,
    Null,
    /// Reserved so that a caller cannot write one: the adapter appends it.
    Limit,
}

const KEYWORDS: [(&str, Keyword); 14] = [
    ("MATCH", Keyword::Match),
    ("WHERE", Keyword::Where),
    ("RETURN", Keyword::Return),
    ("AS", Keyword::As),
    ("ORDER", Keyword::Order),
    ("BY", Keyword::By),
    ("ASC", Keyword::Asc),
    ("DESC", Keyword::Desc),
    ("AND", Keyword::And),
    ("OR", Keyword::Or),
    ("NOT", Keyword::Not),
    ("IS", Keyword::Is),
    ("NULL", Keyword::Null),
    ("LIMIT", Keyword::Limit),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Symbol {
    Open,
    Close,
    EdgeOpen,
    EdgeClose,
    Colon,
    Dot,
    Comma,
    Compare,
}

/// Longest first, so `<=` is not read as `<` followed by `=`.
const SYMBOLS: [(&str, Symbol); 15] = [
    ("<-[", Symbol::EdgeOpen),
    ("]->", Symbol::EdgeClose),
    ("-[", Symbol::EdgeOpen),
    ("]-", Symbol::EdgeClose),
    ("<>", Symbol::Compare),
    ("<=", Symbol::Compare),
    (">=", Symbol::Compare),
    ("(", Symbol::Open),
    (")", Symbol::Close),
    (":", Symbol::Colon),
    (".", Symbol::Dot),
    (",", Symbol::Comma),
    ("=", Symbol::Compare),
    ("<", Symbol::Compare),
    (">", Symbol::Compare),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Keyword(Keyword),
    /// A variable, label, property or alias.
    Name,
    /// A number or a quoted string.
    Literal,
    Symbol(Symbol),
}

/// Splits a query into tokens. Only ASCII whitespace separates them, a string
/// ends at the first quote that matches its opening one, and characters that
/// could start a comment, a parameter or another construct are refused, so
/// every lexer reads the same tokens.
fn graph_tokens(query: &str) -> Result<Vec<Token>, &'static str> {
    let is_space = |character: char| character.is_ascii_whitespace();
    let is_word = |character: char| character.is_ascii_alphanumeric() || character == '_';
    let mut tokens = Vec::new();
    let mut rest = query.trim_start_matches(is_space);
    while let Some(first) = rest.chars().next() {
        let after = if first == '\'' || first == '"' {
            let (text, after) = rest[1..].split_once(first).ok_or(BAD_STRING)?;
            if text.contains('\\') {
                return Err(BAD_STRING);
            }
            tokens.push(Token::Literal);
            after
        } else if first.is_ascii_digit() {
            let (number, after) = rest.split_at(
                rest.find(|character: char| !(is_word(character) || character == '.'))
                    .unwrap_or(rest.len()),
            );
            let parts = number.split('.').collect::<Vec<_>>();
            if parts.len() > 2
                || parts
                    .iter()
                    .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
            {
                return Err(BAD_NUMBER);
            }
            tokens.push(Token::Literal);
            after
        } else if is_word(first) {
            let (word, after) = rest.split_at(
                rest.find(|character: char| !is_word(character))
                    .unwrap_or(rest.len()),
            );
            tokens.push(
                KEYWORDS
                    .iter()
                    .find(|(keyword, _)| word.eq_ignore_ascii_case(keyword))
                    .map_or(Token::Name, |(_, keyword)| Token::Keyword(*keyword)),
            );
            after
        } else {
            let (text, symbol) = SYMBOLS
                .iter()
                .find(|(text, _)| rest.starts_with(text))
                .ok_or(UNKNOWN_CHARACTER)?;
            tokens.push(Token::Symbol(*symbol));
            &rest[text.len()..]
        };
        rest = after.trim_start_matches(is_space);
    }
    Ok(tokens)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Element {
    Node,
    Edge,
}

/// How much of a node or edge has been read after its opening symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Opened,
    Named,
    Colon,
    Labelled,
}

/// Where a variable or property appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Tested,
    Operand,
    Item,
    Sort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Start,
    /// Expects `(`: after `MATCH`, a comma between paths or an edge.
    Node,
    Element(Element, Part),
    /// After `)`: an edge, another path, `WHERE` or `RETURN`.
    AfterNode,
    /// Expects a test: after `WHERE`, `AND`, `OR` or `NOT`.
    Test,
    Value(Slot),
    /// After `.`: expects a property name.
    Property(Slot),
    Operand,
    Is,
    IsNot,
    Tested,
    Item,
    Alias,
    Aliased,
    Order,
    Sort,
    Sorted,
}

/// The next state of the read-only grammar, or `None` when `token` cannot
/// follow `state`.
fn next_state(state: State, token: Token) -> Option<State> {
    use {Keyword as K, State as S, Symbol as Y, Token as T};
    Some(match (state, token) {
        (S::Start, T::Keyword(K::Match))
        | (S::AfterNode, T::Symbol(Y::Comma))
        | (S::Element(Element::Edge, Part::Labelled), T::Symbol(Y::EdgeClose)) => S::Node,
        (S::Node, T::Symbol(Y::Open)) => S::Element(Element::Node, Part::Opened),
        (S::AfterNode, T::Symbol(Y::EdgeOpen)) => S::Element(Element::Edge, Part::Opened),
        (S::Element(element, Part::Opened), T::Name) => S::Element(element, Part::Named),
        (S::Element(element, Part::Opened | Part::Named), T::Symbol(Y::Colon)) => {
            S::Element(element, Part::Colon)
        }
        (S::Element(element, Part::Colon), T::Name) => S::Element(element, Part::Labelled),
        (S::Element(element, Part::Opened | Part::Named), token) => {
            return next_state(S::Element(element, Part::Labelled), token);
        }
        (S::Element(Element::Node, Part::Labelled), T::Symbol(Y::Close)) => S::AfterNode,
        (S::AfterNode, T::Keyword(K::Where))
        | (S::Test, T::Keyword(K::Not))
        | (S::Tested, T::Keyword(K::And | K::Or)) => S::Test,
        (S::Test, T::Name) => S::Value(Slot::Tested),
        (S::Value(slot), T::Symbol(Y::Dot)) => S::Property(slot),
        (S::Property(slot), T::Name) => S::Value(slot),
        (S::Value(Slot::Tested), T::Symbol(Y::Compare)) => S::Operand,
        (S::Value(Slot::Tested), T::Keyword(K::Is)) => S::Is,
        (S::Operand, T::Name) => S::Value(Slot::Operand),
        (S::Operand, T::Literal) | (S::Is | S::IsNot, T::Keyword(K::Null)) => S::Tested,
        (S::Is, T::Keyword(K::Not)) => S::IsNot,
        (S::Value(Slot::Tested | Slot::Operand), token) => return next_state(S::Tested, token),
        (S::AfterNode | S::Tested, T::Keyword(K::Return)) | (S::Aliased, T::Symbol(Y::Comma)) => {
            S::Item
        }
        (S::Item, T::Name) => S::Value(Slot::Item),
        (S::Value(Slot::Item), T::Keyword(K::As)) => S::Alias,
        (S::Alias, T::Name) => S::Aliased,
        (S::Value(Slot::Item), token) => return next_state(S::Aliased, token),
        (S::Aliased, T::Keyword(K::Order)) => S::Order,
        (S::Order, T::Keyword(K::By)) | (S::Sorted, T::Symbol(Y::Comma)) => S::Sort,
        (S::Sort, T::Name) => S::Value(Slot::Sort),
        (S::Value(Slot::Sort), T::Keyword(K::Asc | K::Desc)) => S::Sorted,
        (S::Value(Slot::Sort), token) => return next_state(S::Sorted, token),
        _ => return None,
    })
}

fn validate_read_grammar(query: &str) -> Result<(), &'static str> {
    let mut state = State::Start;
    for token in graph_tokens(query)? {
        state = next_state(state, token).ok_or(if state == State::Start {
            NOT_MATCH
        } else {
            OUTSIDE_GRAMMAR
        })?;
    }
    if matches!(
        state,
        State::Value(Slot::Item | Slot::Sort) | State::Aliased | State::Sorted
    ) {
        Ok(())
    } else {
        Err(OUTSIDE_GRAMMAR)
    }
}

fn invalid_graph_query(reason: &'static str) -> PolyglotError {
    PolyglotError::InvalidIdentifier {
        kind: "graph query",
        reason,
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
