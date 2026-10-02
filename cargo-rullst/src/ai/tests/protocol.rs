use super::*;

fn block(json: &str) -> String {
    format!("{FENCE_START}\n{json}\n```\n")
}

#[test]
fn well_formed_actions_parse_in_order() {
    let response = format!(
        "Plan:\n{}{}{}{}Done.",
        block(r#"{"action":"write_file","path":"src/a.rs","content":"fn a() {}\n"}"#),
        block(r#"{"action":"edit_file","path":"src/main.rs","find":"old","replace":"new"}"#),
        block(r#"{"action":"run_rullst","args":["make:model","Post","--migration"]}"#),
        block(r#"{"action":"cargo","args":["check"]}"#),
    );
    let actions: Vec<Action> = parse(&response).into_iter().map(Result::unwrap).collect();
    assert_eq!(
        actions,
        vec![
            Action::WriteFile {
                path: "src/a.rs".into(),
                content: "fn a() {}\n".into()
            },
            Action::EditFile {
                path: "src/main.rs".into(),
                find: "old".into(),
                replace: "new".into()
            },
            Action::RunRullst {
                args: vec!["make:model".into(), "Post".into(), "--migration".into()]
            },
            Action::Cargo {
                args: vec!["check".into()]
            },
        ]
    );
}

#[test]
fn text_without_fences_and_other_code_blocks_yield_nothing() {
    assert!(parse("Just an answer.").is_empty());
    let response =
        "```rust\nfn main() {}\n```\n```json\n{\"action\":\"cargo\",\"args\":[\"check\"]}\n```\n";
    assert!(parse(response).is_empty());
}

#[test]
fn malformed_and_hostile_blocks_are_rejected_individually() {
    let cases = [
        ("not json", "not valid JSON"),
        ("[1, 2]", "JSON object"),
        (r#"{"path":"x"}"#, "`action`"),
        (
            r#"{"action":"shell","command":"rm -rf /"}"#,
            "unknown action",
        ),
        (
            r#"{"action":"read_file","path":"/etc/passwd"}"#,
            "unknown action",
        ),
        (
            r#"{"action":"write_file","path":"a","content":"b","mode":"755"}"#,
            "unexpected field",
        ),
        (
            r#"{"action":"write_file","path":"a"}"#,
            "`content` is required",
        ),
        (
            r#"{"action":"write_file","path":7,"content":"b"}"#,
            "`path` must be a string",
        ),
        (
            r#"{"action":"edit_file","path":"a","find":"","replace":"b"}"#,
            "must not be empty",
        ),
        (
            r#"{"action":"run_rullst","args":"make:model Post"}"#,
            "array",
        ),
        (r#"{"action":"run_rullst","args":[]}"#, "between 1 and 9"),
        (r#"{"action":"cargo","args":["check", 1]}"#, "short strings"),
        (r#"{"action":"cargo","args":[["nested"]]}"#, "short strings"),
    ];
    for (json, reason) in cases {
        let results = parse(&block(json));
        assert_eq!(results.len(), 1, "{json}");
        let error = results[0].as_ref().unwrap_err();
        assert!(error.contains(reason), "{json}: {error}");
    }
    let deep = format!("{}{}", "[".repeat(5000), "]".repeat(5000));
    assert!(parse(&block(&deep))[0].is_err());
    let huge = format!(
        r#"{{"action":"write_file","path":"a","content":"{}"}}"#,
        "x".repeat(MAX_CONTENT_BYTES + 1)
    );
    assert!(
        parse(&block(&huge))[0]
            .as_ref()
            .unwrap_err()
            .contains("exceeds")
    );
}

#[test]
fn unknown_field_names_are_sanitised_in_errors() {
    let json = "{\"action\":\"cargo\",\"args\":[\"check\"],\"\\u001b[2J\":1}";
    let error = parse(&block(json)).remove(0).unwrap_err();
    assert!(!error.contains('\x1b'));
}

#[test]
fn unterminated_oversized_and_excess_blocks_fail_closed() {
    let results = parse(&format!(
        "{FENCE_START}\n{{\"action\":\"cargo\",\"args\":[\"check\"]}}\n"
    ));
    assert_eq!(results.len(), 1);
    assert!(results[0].as_ref().unwrap_err().contains("not closed"));

    let oversized = format!("{FENCE_START}\n{}\n```\n", "x".repeat(600 * 1024));
    assert!(
        parse(&oversized)[0]
            .as_ref()
            .unwrap_err()
            .contains("512 KiB")
    );

    let many: String = (0..12)
        .map(|_| block(r#"{"action":"cargo","args":["check"]}"#))
        .collect();
    let results = parse(&many);
    assert_eq!(results.len(), MAX_ACTIONS + 1);
    assert!(results[MAX_ACTIONS].is_err());
}

#[test]
fn display_filter_hides_action_blocks_across_chunk_boundaries() {
    let response = format!(
        "Here is the plan.\n```rust\nlet x = 1;\n```\n{}Bye",
        block(r#"{"action":"cargo","args":["check"]}"#)
    );
    for size in [1, 2, 3, 7, 64] {
        let mut filter = DisplayFilter::default();
        let mut shown = String::new();
        let chars: Vec<char> = response.chars().collect();
        for chunk in chars.chunks(size) {
            shown.push_str(&filter.push(&chunk.iter().collect::<String>()));
        }
        shown.push_str(&filter.finish());
        assert_eq!(
            shown, "Here is the plan.\n```rust\nlet x = 1;\n```\nBye",
            "chunk {size}"
        );
        assert_eq!(filter.hidden_blocks, 1);
    }
    // Blank lines around hidden blocks collapse.
    let response = format!("Plan:\n\n{}\n{}\nDone\n", block("{}"), block("{}"));
    let mut filter = DisplayFilter::default();
    let shown = filter.push(&response) + &filter.finish();
    assert_eq!(shown, "Plan:\n\nDone\n");
    assert_eq!(filter.hidden_blocks, 2);
}

#[test]
fn display_filter_sanitises_terminal_controls() {
    let mut filter = DisplayFilter::default();
    let shown = filter.push("ok\x1b]0;title\x07\n");
    assert!(!shown.contains('\x1b'));
    assert!(shown.contains("\\u{1b}"));
}

#[test]
fn display_filter_is_linear_on_long_whitespace_lines() {
    // Indented fences are still hidden.
    let mut filter = DisplayFilter::default();
    let response = format!("a\n\t  {}b\n", block("{}").replace('\n', "\n  "));
    let shown = filter.push(&response) + &filter.finish();
    assert_eq!(filter.hidden_blocks, 1, "{shown:?}");
    assert!(
        shown.starts_with("a\n") && shown.ends_with("b\n"),
        "{shown:?}"
    );

    // 1 MiB of whitespace without a newline, in small chunks, then text: each
    // character used to rescan the whole held-back line.
    let started = std::time::Instant::now();
    let mut filter = DisplayFilter::default();
    let mut shown = String::new();
    let spaces = " \t".repeat(256);
    for _ in 0..2048 {
        shown.push_str(&filter.push(&spaces));
    }
    assert!(shown.is_empty(), "whitespace could still start a fence");
    shown.push_str(&filter.push("text\n"));
    assert_eq!(shown.len(), 1024 * 1024 + 5);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
}
