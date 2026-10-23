use super::*;

fn terminal() -> TerminalEnv {
    TerminalEnv {
        stdin_tty: true,
        stdout_tty: true,
        stderr_tty: true,
        ..TerminalEnv::default()
    }
}

#[test]
fn only_an_unrestricted_terminal_animates() {
    assert_eq!(spinner_mode(&terminal()), SpinnerMode::Animated);
    type Restriction = (&'static str, fn(&mut TerminalEnv));
    let restricted: [Restriction; 5] = [
        ("pipe or CI log", |env| env.stdout_tty = false),
        ("NO_COLOR", |env| env.no_color = true),
        ("CI", |env| env.ci = true),
        ("TERM=dumb", |env| env.dumb = true),
        ("RULLST_REDUCED_MOTION", |env| env.reduced_motion = true),
    ];
    for (name, restrict) in restricted {
        let mut env = terminal();
        restrict(&mut env);
        assert_eq!(spinner_mode(&env), SpinnerMode::Plain, "{name}");
    }
    // Only standard output, which the spinner draws on, decides.
    let mut env = terminal();
    env.stdin_tty = false;
    env.stderr_tty = false;
    assert_eq!(spinner_mode(&env), SpinnerMode::Animated);
}

#[test]
fn plain_progress_is_one_line_without_escape_sequences() {
    let mut out = Vec::new();
    let result = plain_progress(&mut out, "Applying compiler-provided migrations...", || 42);
    assert_eq!(result, 42);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Applying compiler-provided migrations...\n"
    );
}
