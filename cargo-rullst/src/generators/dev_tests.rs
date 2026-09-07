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
