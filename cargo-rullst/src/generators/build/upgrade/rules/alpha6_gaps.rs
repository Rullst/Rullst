//! Rules for the 13.0.0-alpha.6 changes that the first alpha.6 rules left to
//! the checklist: cron expressions that Core's evaluator now refuses, code
//! that compares the masked CSRF token with its cookie, `jsonwebtoken` key
//! constructors that return a `Result` in 12, and the session-epoch calls
//! that `cargo rullst auth`, the SaaS starter, `make:mfa` and `make:oauth`
//! generated before alpha.6. [`super::tokens`] calls them with what it reads;
//! the dependency and `Rullst.toml` rules are in `files/alpha6.rs`, and the
//! `new` of a `define_provider!` type is resolved per package by
//! [`super::Collector`].

use super::catalog::{
    AUTH_SESSION_EPOCH_CALLS, AUTH_SESSION_REGISTRY, JSONWEBTOKEN_12, SCHEDULER_CRON_FIELDS,
};
use super::rust::FileContext;
use super::tokens::string_value;
use super::{Rule, WorkspaceFacts};
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// The `Scheduler` methods whose first argument is a cron expression.
const CRON_METHODS: [&str; 4] = [
    "task",
    "task_in_tz",
    "deferrable_task",
    "deferrable_task_in_tz",
];

/// Whether Core's evaluator refuses `expression` for the reason
/// 13.0.0-alpha.6 introduced: a whitespace-separated field that starts or
/// ends with `,`, `-` or `/`, which the `cron` crate joined with its
/// neighbour. Expressions refused for another reason were refused before.
pub(super) fn refused_cron(expression: &str) -> bool {
    let separators = [',', '-', '/'];
    let split_field = expression
        .split_whitespace()
        .any(|field| field.starts_with(separators) || field.ends_with(separators));
    split_field
        && rullst_core::Scheduler::new()
            .task(expression, || async {})
            .is_err()
}

/// The rule for a method call `.name(args)`: a `Scheduler` registration
/// whose cron expression, a string literal, Core's evaluator refuses, in a
/// file that names `Scheduler`.
pub(super) fn cron_rule(name: &str, args: &TokenStream, text: &str) -> Option<&'static Rule> {
    if !CRON_METHODS.contains(&name) || !text.contains("Scheduler") {
        return None;
    }
    let expression = string_value(args.clone().into_iter().next().as_ref())?;
    refused_cron(&expression).then_some(&SCHEDULER_CRON_FIELDS)
}

/// The rule for a path such as `EncodingKey::from_secret`, if any: a
/// `jsonwebtoken` key constructor in a workspace that declares
/// `jsonwebtoken` 11 or earlier.
pub(super) fn path_rule(segments: &[String], context: &FileContext<'_>) -> Option<&'static Rule> {
    match segments {
        [.., key, function]
            if function == "from_secret"
                && (key == "EncodingKey" || key == "DecodingKey")
                && context.facts.jsonwebtoken_below_12 =>
        {
            Some(&JSONWEBTOKEN_12)
        }
        _ => None,
    }
}

/// The rule for a call of `segments`, if any: `make_login_cookie_with_epoch`
/// or `decrypt_session_claims`, which issue and read session-epoch cookies,
/// in any file of the package. Without a session registry feature the call
/// is where the registry would take over (`V13-AUTH-SESSION-REGISTRY`); with
/// one, the registry's middleware accepts only registered sessions, which
/// `decrypt_session_claims` refuses, so the call fails at run time
/// (`V13-AUTH-SESSION-EPOCH-CALLS`). Both are rules about production
/// behaviour, so test code is skipped ([`super::test_code`]).
pub(super) fn session_call_rule(
    segments: &[&str],
    facts: &WorkspaceFacts,
) -> Option<&'static Rule> {
    let epoch_call = matches!(
        segments.last(),
        Some(&("make_login_cookie_with_epoch" | "decrypt_session_claims"))
    );
    match (epoch_call, facts.session_registry) {
        (true, true) => Some(&AUTH_SESSION_EPOCH_CALLS),
        (true, false) => Some(&AUTH_SESSION_REGISTRY),
        (false, _) => None,
    }
}

/// What the identifiers and string literals of an operand mention.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Mentions {
    /// An identifier containing `cookie`.
    cookie: bool,
    /// A string literal containing `rullst_csrf=`, the cookie's name.
    cookie_name: bool,
    /// An identifier containing `token` or `csrf`.
    token: bool,
    /// An identifier or string literal containing `csrf`.
    csrf: bool,
}

impl Mentions {
    fn of<'a>(tokens: impl IntoIterator<Item = &'a TokenTree>) -> Self {
        let mut mentions = Self::default();
        for tree in tokens {
            mentions.read(tree);
        }
        mentions
    }

    fn read(&mut self, tree: &TokenTree) {
        match tree {
            TokenTree::Ident(ident) => {
                let name = ident.to_string().to_ascii_lowercase();
                self.cookie |= name.contains("cookie");
                self.token |= name.contains("token") || name.contains("csrf");
                self.csrf |= name.contains("csrf");
            }
            TokenTree::Literal(_) => {
                if let Some(value) = string_value(Some(tree)) {
                    let value = value.to_ascii_lowercase();
                    self.cookie_name |= value.contains("rullst_csrf=");
                    self.csrf |= value.contains("csrf");
                }
            }
            TokenTree::Group(group) => {
                for tree in group.stream() {
                    self.read(&tree);
                }
            }
            TokenTree::Punct(_) => {}
        }
    }

    fn reads_cookie(self) -> bool {
        self.cookie || self.cookie_name
    }

    /// One operand reads the cookie and the other is a token that does not:
    /// the CSRF cookie, unless neither operand nor the file names CSRF.
    fn compared_with(self, other: Self, file_names_csrf: bool) -> bool {
        let pair = |cookie: Self, token: Self| {
            cookie.reads_cookie() && token.token && !token.reads_cookie()
        };
        (pair(self, other) || pair(other, self)) && (self.csrf || other.csrf || file_names_csrf)
    }
}

fn punct(tree: Option<&TokenTree>, expected: char) -> bool {
    matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == expected)
}

/// `==` at `index`, which never compares a masked token with the cookie
/// successfully. `!=` and `assert_ne!` are what 13.0.0-alpha.6 expects.
fn equality(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Punct(first)) if first.as_char() == '=' && first.spacing() == Spacing::Joint)
        && punct(tokens.get(index + 1), '=')
}

/// Whether the token at `index` ends an operand: a separator, an
/// assignment, a `=>`, a logical operator, a block or a keyword that starts a
/// clause. The operator being read is never walked over, and the second `=`
/// of another comparison cannot appear in an operand (Rust does not chain
/// comparisons).
fn boundary(tokens: &[TokenTree], index: usize) -> bool {
    match &tokens[index] {
        TokenTree::Punct(punct) => match punct.as_char() {
            ',' | ';' => true,
            // `&&` and `||` start with a joint `&` or `|`; `&value` does not.
            '&' | '|' => punct.spacing() == Spacing::Joint,
            // An assignment; the first `=` of `==` or `=>` is joint.
            '=' => punct.spacing() == Spacing::Alone,
            // The `>` of `=>`.
            '>' => index
                .checked_sub(1)
                .is_some_and(|before| punct_at(tokens, before, '=')),
            _ => false,
        },
        TokenTree::Group(group) => group.delimiter() == Delimiter::Brace,
        TokenTree::Ident(ident) => matches!(
            ident.to_string().as_str(),
            "if" | "while" | "let" | "return" | "match" | "else" | "in"
        ),
        TokenTree::Literal(_) => false,
    }
}

fn punct_at(tokens: &[TokenTree], index: usize, expected: char) -> bool {
    punct(tokens.get(index), expected)
}

/// The tokens of the operands on either side of the operator at
/// `start..end`.
fn operands(tokens: &[TokenTree], start: usize, end: usize) -> (Mentions, Mentions) {
    let first = (0..start)
        .rev()
        .find(|index| boundary(tokens, *index))
        .map_or(0, |index| index + 1);
    let last = (end..tokens.len())
        .find(|index| boundary(tokens, *index))
        .unwrap_or(tokens.len());
    (
        Mentions::of(&tokens[first..start]),
        Mentions::of(&tokens[end..last]),
    )
}

/// Whether a token continues a method's receiver chain backwards: a name, a
/// call's or index's group, `.`, `?` or a path's `::`.
fn chain_link(tree: &TokenTree) -> bool {
    match tree {
        TokenTree::Ident(ident) => !matches!(
            ident.to_string().as_str(),
            "if" | "let" | "return" | "while" | "match" | "in"
        ),
        TokenTree::Group(group) => group.delimiter() != Delimiter::Brace,
        TokenTree::Punct(link) => matches!(link.as_char(), '.' | '?' | ':'),
        TokenTree::Literal(_) => true,
    }
}

/// The receiver chain of a method called at `dot` (the `.` before its name),
/// and whether `!` negates it.
fn receiver(tokens: &[TokenTree], dot: usize) -> (Mentions, bool) {
    let first = (0..dot)
        .rev()
        .find(|index| !chain_link(&tokens[*index]))
        .map_or(0, |index| index + 1);
    let negated = first.checked_sub(1).is_some_and(|before| {
        matches!(&tokens[before], TokenTree::Punct(p) if p.as_char() == '!' && p.spacing() == Spacing::Alone)
    });
    (Mentions::of(&tokens[first..dot]), negated)
}

/// The arguments of a macro body split at its top-level commas.
fn arguments(tokens: &[TokenTree]) -> Vec<&[TokenTree]> {
    tokens
        .split(|tree| matches!(tree, TokenTree::Punct(p) if p.as_char() == ','))
        .collect()
}

fn line(tree: &TokenTree) -> usize {
    tree.span().start().line
}

/// The lines in `tokens`, one level of a file's token stream (`macro_name`
/// names the macro whose body it is), where code checks a CSRF token against
/// the `rullst_csrf` cookie's value: an `==`, `assert_eq!` or
/// `debug_assert_eq!` whose operands are a cookie value and another token,
/// a `.contains(..)` of a cookie value on something else, such as a page
/// body, that `!` does not negate, and `.eq(..)` or `.ct_eq(..)` between
/// them. The operands, the arguments or the file must name CSRF.
pub(super) fn csrf_comparisons(
    tokens: &[TokenTree],
    macro_name: Option<&str>,
    text: &str,
) -> Vec<usize> {
    let file_names_csrf = text.contains("rullst_csrf") || text.contains("CsrfToken");
    let mut found = Vec::new();
    let arguments = arguments(tokens);
    if matches!(macro_name, Some("assert_eq" | "debug_assert_eq"))
        && let [left, right, ..] = arguments.as_slice()
        && Mentions::of(*left).compared_with(Mentions::of(*right), file_names_csrf)
        && let Some(first) = left.first()
    {
        found.push(line(first));
    }
    for index in 0..tokens.len() {
        if equality(tokens, index) {
            let (left, right) = operands(tokens, index, index + 2);
            if left.compared_with(right, file_names_csrf) {
                found.push(line(&tokens[index]));
            }
            continue;
        }
        let (Some(TokenTree::Ident(method)), Some(TokenTree::Group(args))) =
            (tokens.get(index + 1), tokens.get(index + 2))
        else {
            continue;
        };
        if !punct(tokens.get(index), '.') || args.delimiter() != Delimiter::Parenthesis {
            continue;
        }
        let argument = Mentions::of(&args.stream().into_iter().collect::<Vec<_>>());
        let (receiver, negated) = receiver(tokens, index);
        let compared = match method.to_string().as_str() {
            // A page that contains the cookie's value; `rullst_csrf=` alone
            // looks for the cookie in a `Set-Cookie` header instead.
            "contains" => {
                !negated
                    && argument.cookie
                    && !receiver.reads_cookie()
                    && (argument.csrf || file_names_csrf)
            }
            "eq" | "ct_eq" => !negated && receiver.compared_with(argument, file_names_csrf),
            _ => false,
        };
        if compared {
            found.push(method.span().start().line);
        }
    }
    found
}
