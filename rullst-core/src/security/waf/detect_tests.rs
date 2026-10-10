//! Direct checks of the structural detector on decoded, lowercased text.

use super::*;

#[test]
fn collapse_merges_whitespace_and_strips_only_closed_comments() {
    assert_eq!(collapse(b"a \t\n b", false), b"a b");
    assert_eq!(collapse(b"a/* x */b", false), b"a/* x */b");
    assert_eq!(collapse(b"a/* x */b", true), b"a b");
    assert_eq!(collapse(b"a/**/ /**/b", true), b"a b");
    assert_eq!(collapse(b"/*1*/x/*22*/y", true), b" x y");
    // An unclosed comment is kept, and so is every later one.
    assert_eq!(collapse(b"a /* b /* c", true), b"a /* b /* c");
    assert_eq!(collapse(b"", true), b"");
}

#[test]
fn runs_end_where_the_predicate_stops() {
    assert_eq!(skip_spaces(b"a   b", 1), 4);
    assert_eq!(skip_spaces(b"ab", 1), 1);
    assert_eq!(skip_spaces(b"a  ", 1), 3);
    assert_eq!(skip_spaces(b"a", 5), 5);
    assert_eq!(run_end(b"x12y", 1, |byte| byte.is_ascii_digit()), 3);
}

#[test]
fn a_comment_that_hides_nothing_does_not_flag_prose() {
    assert!(!is_injection("see /* the note */ below"));
    assert!(is_injection("1 union/**/select password"));
}

#[test]
fn only_double_ampersand_and_dollar_parenthesis_start_a_command() {
    assert!(is_injection("x && cat /etc/passwd"));
    assert!(is_injection("&&id"));
    assert!(!is_injection("tom & cat"));
    assert!(!is_injection("a&cat=2"));
    assert!(is_injection("$(whoami)"));
    assert!(is_injection("x=$(id)"));
    assert!(!is_injection("$5 ls"));
    assert!(!is_injection("costs $ cat"));
}

#[test]
fn command_names_may_carry_a_version_but_not_more_letters() {
    assert!(is_injection("; python3 -c 1"));
    assert!(is_injection("| php8"));
    assert!(is_injection("; /usr/bin/curl x"));
    assert!(!is_injection("; catalog"));
    assert!(!is_injection("; python3x"));
}

#[test]
fn quote_breakouts_skip_spaces_and_closing_parentheses() {
    assert!(is_injection("x') or 1=1"));
    assert!(is_injection("x' ) ) or 'a'='a"));
    assert!(!is_injection("it's ) or so"));
}

#[test]
fn union_select_allows_parentheses_and_set_quantifiers() {
    assert!(is_injection("1 union (select 1"));
    assert!(is_injection("1 union all select 1"));
    assert!(is_injection("1 union  distinct ( select 1"));
    assert!(!is_injection("trade union selection"));
}
