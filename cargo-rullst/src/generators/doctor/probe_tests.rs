//! Probes report found/success and a bounded, cleaned first output line.
use super::*;

fn bounded(program: &'static str, args: &'static [&'static str]) -> Probe {
    // Bounded, so a truncation loop that never ends fails instead of hanging.
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(run(program, args));
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the probe finished")
}

#[cfg(unix)]
#[test]
fn probes_keep_the_first_nonempty_line_without_control_characters() {
    let probe = bounded(
        "sh",
        &["-c", "printf '\\n  \\a\\033first line  \\nsecond\\n'"],
    );
    assert!(probe.found && probe.success);
    assert_eq!(probe.first_line, "first line");
    assert!(probe.stdout.ends_with("second\n"));
    let failed = bounded("sh", &["-c", "echo failed; exit 2"]);
    assert!(failed.found && !failed.success);
    assert_eq!(
        bounded("rullst-missing-probe-program", &[]),
        Probe::default()
    );
}

#[cfg(unix)]
#[test]
fn probe_output_is_bounded_at_a_character_boundary() {
    let long = bounded("sh", &["-c", "head -c 70000 /dev/zero | tr '\\000' a"]);
    assert_eq!(long.stdout.len(), 65_536);
    // 65,535 ASCII bytes then a two-byte character straddling the bound.
    let straddling = bounded(
        "sh",
        &[
            "-c",
            "head -c 65535 /dev/zero | tr '\\000' a; printf '\\303\\251'",
        ],
    );
    assert_eq!(straddling.stdout.len(), 65_535);
}

#[cfg(unix)]
#[test]
fn installed_tools_are_found_on_the_search_path() {
    let shell = installed("sh");
    assert!(shell.found && shell.success && shell.stdout.is_empty());
    assert!(!installed("rullst-missing-probe-program").found);
    let probes = run_all(&[
        ("sh", vec!["-c", "echo one"]),
        ("sh", vec!["-c", "echo two"]),
    ]);
    assert_eq!(
        probes
            .iter()
            .map(|probe| probe.first_line.as_str())
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
}

#[test]
fn the_cargo_version_is_a_numeric_token() {
    let version = tool_version("cargo").unwrap();
    assert!(version.starts_with("1."), "{version}");
}
