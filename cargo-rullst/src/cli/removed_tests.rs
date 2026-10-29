use super::*;

#[test]
fn only_ai_is_removed() {
    assert!(is_removed("ai"));
    for name in [
        "green",
        "footprint",
        "Ai",
        "ai:x",
        "generate:ai-context",
        "",
    ] {
        assert!(!is_removed(name), "{name}");
    }
    let names: Vec<String> = commands()
        .map(|command| command.get_name().to_string())
        .collect();
    assert_eq!(names, ["ai"]);
}

#[test]
fn removed_commands_are_hidden_and_take_any_arguments() {
    let command = crate::command();
    let ai = command.find_subcommand("ai").expect("the ai stub");
    assert!(ai.is_hide_set());
    let matches = command
        .try_get_matches_from(["rullst", "-v", "ai", "--help", "fix", "--json"])
        .expect("anything after `ai` parses");
    assert_eq!(matches.subcommand_name(), Some("ai"));
    let (_, ai) = matches.subcommand().unwrap();
    let arguments: Vec<&String> = ai.get_many::<String>("arguments").unwrap().collect();
    assert_eq!(arguments, ["--help", "fix", "--json"]);

    let names = crate::cli::suggest::command_names(&crate::command());
    assert!(!names.iter().any(|name| name == "ai"), "{names:?}");
}

#[test]
fn the_report_names_the_release_the_replacement_and_the_migration_section() {
    let removed = find("ai").expect("ai is listed");
    let report = friendly(removed);
    assert_eq!(report.title, "`cargo rullst ai` was removed");
    assert_eq!(
        report.happened,
        "`cargo rullst ai` was removed in 13.0.0-alpha.4."
    );
    let fix = report.fix.unwrap_or_default();
    assert!(fix.contains("AGENTS.md"), "{fix}");
    assert!(fix.contains("cargo rullst generate:ai-context"), "{fix}");
    assert!(
        fix.contains("https://rullst.github.io/Rullst/llms.txt"),
        "{fix}"
    );
    assert_eq!(
        report.docs,
        Some(
            "https://rullst.github.io/Rullst/book/migration-v13.html#upgrading-an-alpha3-application-to-alpha4"
        )
    );
}

#[test]
fn running_a_removed_command_fails_with_status_1() {
    let error = report("ai");
    assert_eq!(error.to_string(), "`cargo rullst ai` was removed");
    let error: &(dyn Error + 'static) = error.as_ref();
    assert_eq!(error_report::report(error, &[]), 1);
}
