//! Tests of the console output and the access log.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn development_polls_are_not_access_logged() {
    assert!(!is_logged("/_rullst/dev-generation"));
    assert!(!is_logged("/_rullst/dev-reload.js"));
    assert!(!is_logged("/_rullst/dev-telemetry"));
    assert!(!is_logged("/_rullst_hmr"));
    assert!(is_logged("/orders"));
    assert!(is_logged("/_rullst/dev-generation/extra"));
}

#[test]
fn quiet_health_probes_still_log_failures_and_other_paths() {
    for path in ["/health", "/ready"] {
        assert!(!should_log(path, 200, false), "{path}");
        assert!(!should_log(path, 399, false), "{path}");
        assert!(should_log(path, 400, false), "{path}");
        assert!(should_log(path, 503, false), "{path}");
        assert!(should_log(path, 200, true), "{path}");
    }
    assert!(should_log("/health/details", 200, false));
    assert!(should_log("/orders", 200, false));
    assert!(!should_log("/_rullst/dev-generation", 500, true));
}

#[test]
#[allow(clippy::unwrap_used)]
fn health_probe_logging_follows_the_environment_unless_configured() {
    let none = HashMap::new();
    let dotenv =
        |value: &str| HashMap::from([("RULLST_LOG_HEALTH_PROBES".to_string(), value.to_string())]);
    assert_eq!(LOG_HEALTH_PROBES_VARIABLE, "RULLST_LOG_HEALTH_PROBES");
    for (environment, logged) in [
        (Environment::Development, true),
        (Environment::Test, true),
        (Environment::Staging, false),
        (Environment::Production, false),
    ] {
        assert_eq!(log_health_probes(environment, None, &none).unwrap(), logged);
    }
    let production = Environment::Production;
    assert!(log_health_probes(production, Some("true".into()), &none).unwrap());
    assert!(log_health_probes(production, None, &dotenv("on")).unwrap());
    assert!(!log_health_probes(production, Some("0".into()), &dotenv("1")).unwrap());
    assert!(!log_health_probes(Environment::Development, None, &dotenv("false")).unwrap());
    match log_health_probes(production, Some("sometimes".into()), &none) {
        Err(ServerError::Configuration(message)) => {
            assert_eq!(message, "RULLST_LOG_HEALTH_PROBES must be true or false");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
#[allow(clippy::unwrap_used)]
fn access_log_recording_follows_rullst_access_log() {
    use std::num::NonZeroU32;
    let none = HashMap::new();
    let dotenv =
        |value: &str| HashMap::from([("RULLST_ACCESS_LOG".to_string(), value.to_string())]);
    let parse = |value: &str| access_log_recording(Some(value.to_string()), &none);
    assert_eq!(ACCESS_LOG_VARIABLE, "RULLST_ACCESS_LOG");
    assert_eq!(access_log_recording(None, &none).unwrap(), Recording::All);
    for value in ["on", "TRUE", " 1 ", "yes", "1/1"] {
        assert_eq!(parse(value).unwrap(), Recording::All, "{value}");
    }
    for value in ["off", "False", "0", "no"] {
        assert_eq!(parse(value).unwrap(), Recording::Off, "{value}");
    }
    assert_eq!(parse(" Errors ").unwrap(), Recording::Errors);
    let sample = |interval| Recording::Sample(NonZeroU32::new(interval).unwrap());
    assert_eq!(parse("1/2").unwrap(), sample(2));
    assert_eq!(parse("1/100").unwrap(), sample(100));
    assert_eq!(parse("1/1000000").unwrap(), sample(1_000_000));
    // The process value wins over `.env`.
    assert_eq!(
        access_log_recording(Some("errors".into()), &dotenv("off")).unwrap(),
        Recording::Errors
    );
    assert_eq!(
        access_log_recording(None, &dotenv("1/10")).unwrap(),
        sample(10)
    );
    for value in [
        "sometimes",
        "1/0",
        "1/1000001",
        "2/10",
        "1/x",
        "1/",
        "",
        "error",
    ] {
        match parse(value) {
            Err(ServerError::Configuration(message)) => assert_eq!(
                message,
                "RULLST_ACCESS_LOG must be on, off, errors or 1/N with N from 1 to 1000000",
                "{value}"
            ),
            other => panic!("unexpected result for {value:?}: {other:?}"),
        }
    }
}

#[test]
fn an_access_log_duration_has_two_decimals_rounded_half_up() {
    let shown =
        |micros: u64| Milliseconds::of(std::time::Duration::from_micros(micros)).to_string();
    assert_eq!(shown(1_500_250), "1500.25");
    assert_eq!(shown(1_000_050), "1000.05");
    assert_eq!(shown(1_234_567), "1234.57");
    assert_eq!(shown(999_995), "1000.00");
    assert_eq!(
        (shown(4), shown(5), shown(0)),
        ("0.00".into(), "0.01".into(), "0.00".into())
    );
    // Nanoseconds below a microsecond do not count.
    assert_eq!(
        Milliseconds::of(std::time::Duration::from_nanos(14_999)).to_string(),
        "0.01"
    );
    assert_eq!(
        Milliseconds::of(std::time::Duration::MAX).to_string(),
        "18446744073709551.61"
    );
}

#[cfg(unix)]
#[path = "console_unix_tests.rs"]
mod unix;

/// A line names the framework layer that answered the request (#845).
#[test]
fn an_access_line_names_the_refusing_layer() {
    use super::super::access_log::Refusal;
    let elapsed = Milliseconds::of(std::time::Duration::from_micros(310));
    assert_eq!(
        access_line(
            "GET",
            "/",
            (200, None),
            Milliseconds::of(std::time::Duration::from_micros(1_200))
        ),
        "[HTTP] GET / -> 200 (1.20 ms)"
    );
    for (refusal, layer) in [
        (Refusal::CSRF, "csrf"),
        (Refusal::MACHINE_ENDPOINT, "machine-endpoint"),
        (Refusal::WAF, "waf"),
        (Refusal::RATE_LIMIT, "rate-limit"),
        (Refusal::TRAFFIC_SHIELD, "traffic-shield"),
        (Refusal::LIFECYCLE, "lifecycle"),
    ] {
        assert_eq!(refusal.layer(), layer);
        assert_eq!(
            access_line(
                "POST",
                "/login",
                (403, Some(refusal)),
                Milliseconds(elapsed.0)
            ),
            format!("[HTTP] POST /login -> 403 (0.31 ms) refused by {layer}")
        );
    }
}

#[test]
fn a_refused_response_carries_its_layer() {
    use super::super::access_log::{Refusal, refused};
    use axum::response::IntoResponse;
    let response = refused(
        (axum::http::StatusCode::FORBIDDEN, "no").into_response(),
        Refusal::WAF,
    );
    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    assert_eq!(response.extensions().get::<Refusal>(), Some(&Refusal::WAF));
}
