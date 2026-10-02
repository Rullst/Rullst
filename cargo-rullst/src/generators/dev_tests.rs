#![allow(clippy::expect_used)]

use super::*;

fn config(value: Option<i64>) -> toml::Value {
    value.map_or_else(
        || toml::Value::Table(Default::default()),
        |port| toml::from_str(&format!("[app]\nport = {port}\n")).expect("test config"),
    )
}

#[test]
fn configured_port_resolution_has_explicit_precedence_and_bounds() {
    let dotenv = std::collections::HashMap::from([("PORT".to_string(), "4100".to_string())]);
    assert_eq!(
        resolve_configured_port(Some("4200".into()), &dotenv, &config(Some(4_000)))
            .expect("process port"),
        4_200
    );
    assert_eq!(
        resolve_configured_port(None, &dotenv, &config(Some(4_000))).expect("dotenv port"),
        4_100
    );
    assert_eq!(
        resolve_configured_port(None, &Default::default(), &config(Some(4_000)))
            .expect("config port"),
        4_000
    );
    assert_eq!(
        resolve_configured_port(None, &Default::default(), &config(None)).expect("default port"),
        3_000
    );

    for invalid in ["0", "65536", "not-a-port"] {
        let error =
            resolve_configured_port(Some(invalid.into()), &Default::default(), &config(None))
                .expect_err("invalid ports fail closed");
        assert!(error.to_string().contains("between 1 and 65535"));
    }
}

#[cfg(unix)]
#[test]
fn configured_port_rejects_non_unicode_process_values() {
    use std::os::unix::ffi::OsStringExt;

    let error = resolve_configured_port(
        Some(std::ffi::OsString::from_vec(vec![0xff])),
        &Default::default(),
        &config(None),
    )
    .expect_err("non-Unicode port fails closed");
    assert!(error.to_string().contains("not Unicode"));
}

#[test]
fn dashboard_reporting_is_bounded_by_channel_capacity() {
    let (logs, mut receiver) = mpsc::channel(1);
    report(&logs, true, "first".to_string());
    report(&logs, true, "discarded while full".to_string());
    assert!(matches!(
        receiver.try_recv(),
        Ok(LogMsg::System(message)) if message == "first"
    ));

    report(&logs, false, "plain stderr report".to_string());
    assert!(receiver.try_recv().is_err());
}

#[test]
fn configuration_parse_errors_never_echo_file_content() {
    let canary = "sk_live_dev_redaction_canary";
    for (content, line) in [
        (format!("[app]\nport = 3000\napp_key = \"{canary}\n"), 3),
        (
            format!("[database]\nurl = \"postgres://owner:{canary}@db\" trailing\n"),
            2,
        ),
    ] {
        let raw = toml::from_str::<toml::Value>(&content).expect_err("malformed TOML");
        assert!(
            raw.to_string().contains(canary),
            "fixture must exercise the leak"
        );
        let error = parse_rullst_toml(&content).expect_err("malformed Rullst.toml");
        let rendered = format!("{error} {error:?}");
        assert!(!rendered.contains(canary), "{rendered}");
        assert!(!rendered.contains("postgres://"), "{rendered}");
        assert!(
            rendered.starts_with(&format!(
                "Rullst.toml is not valid TOML at line {line}, column "
            )),
            "{rendered}"
        );
    }

    for (content, entry) in [
        (
            format!("PORT=4000\nDATABASE_URL=\"postgres://owner:{canary}@db\nAPP_KEY=x\n"),
            2,
        ),
        (format!("# comment\n\nPORT=4000\nAPP KEY={canary}\n"), 2),
    ] {
        let raw = dotenvy::from_read_iter(content.as_bytes())
            .find_map(Result::err)
            .expect("malformed .env");
        assert!(
            raw.to_string().contains(canary),
            "fixture must exercise the leak"
        );
        let error = parse_dotenv(content.as_bytes()).expect_err("malformed .env");
        let rendered = format!("{error} {error:?}");
        assert!(!rendered.contains(canary), "{rendered}");
        assert!(!rendered.contains("postgres://"), "{rendered}");
        assert_eq!(
            error.to_string(),
            format!("invalid .env syntax in entry {entry}")
        );
    }
    let parsed = parse_dotenv(b"# comment\nPORT=4100\nPORT=4200\n").expect("valid .env");
    assert_eq!(parsed.get("PORT").map(String::as_str), Some("4200"));
}

#[test]
fn ts_sync_reports_a_failed_generation_instead_of_exiting() {
    // `generate:ts` exits the process without route sources; the supervisor
    // must report it and keep running.
    let project = tempfile::tempdir().expect("project");
    let message = typescript_sync_message(project.path());
    assert!(
        message.starts_with("TypeScript SDK sync failed"),
        "{message}"
    );

    std::fs::create_dir_all(project.path().join("src")).expect("source directory");
    std::fs::write(
        project.path().join("src/lib.rs"),
        "routes! { get(\"/teams\" => teams::index) }\n",
    )
    .expect("routes");
    let message = typescript_sync_message(project.path());
    assert!(
        message.starts_with("TypeScript SDK synchronized"),
        "{message}"
    );
    let sdk = std::fs::read_to_string(project.path().join("rullst-client.ts")).expect("SDK");
    assert!(sdk.contains("/teams"), "{sdk}");
}

#[cfg(unix)]
#[tokio::test]
async fn restart_replaces_the_owned_process_with_the_same_snapshot() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("fixture directory");
    let pids = temp.path().join("pids");
    let script = temp.path().join("app.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho $$ >> '{}'\nexec sleep 60\n",
            pids.display()
        ),
    )
    .expect("fixture script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("executable fixture");
    let started = |count: usize| {
        let pids = pids.clone();
        async move {
            for _ in 0..200 {
                let lines = std::fs::read_to_string(&pids)
                    .unwrap_or_default()
                    .lines()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if lines.len() >= count {
                    return lines;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            panic!("the fixture did not start {count} time(s)");
        }
    };
    let alive = |pid: &str| {
        std::process::Command::new("kill")
            .args(["-0", pid])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };

    let mut running = process::Application::prepare(&script).expect("snapshot");
    let (logs, _receiver) = mpsc::channel(8);
    let (status, status_rx) = watch::channel(DevState {
        status: DevStatus::Ready,
        ..DevState::default()
    });
    running.start(false, &logs).expect("first start");
    let first = started(1).await;

    restart(&mut running, false, &logs, &status).expect("restart");
    let both = started(2).await;
    assert_ne!(both[0], both[1]);
    assert_eq!(both[0], first[0]);
    assert!(!alive(&both[0]), "the previous process was not stopped");
    assert!(alive(&both[1]), "the restarted process is not running");
    assert!(matches!(status_rx.borrow().status, DevStatus::Starting));
    assert!(running.try_wait().expect("status").is_none());
    running.stop().expect("stop");
}
