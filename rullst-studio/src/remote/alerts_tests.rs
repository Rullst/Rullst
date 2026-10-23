//! Alert rules with a fixed clock (`TM-STUDIO-3`, `REMOTE-10`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::alerts::{AlertRules, RuleKind, evaluate};
use super::history::Sample;

const NOW: u64 = 1_800_000_000;

fn sample(at: u64) -> Sample {
    Sample {
        at,
        reachable: true,
        requests_per_second: Some(2.0),
        window_requests: Some(40),
        error_rate_percent: Some(10.0),
        p95_ms: Some(150.0),
        queue_pending: Some(12),
        queue_failed_total: Some(15),
    }
}

fn rules() -> AlertRules {
    AlertRules {
        unreachable: true,
        error_rate_percent: Some(5.0),
        p95_ms: Some(100.0),
        queue_pending: Some(10),
        queue_failures: Some(5),
        cooldown_minutes: 15,
    }
}

fn kinds(firings: &[super::alerts::Firing]) -> Vec<RuleKind> {
    firings.iter().map(|firing| firing.rule).collect()
}

#[test]
fn every_threshold_fires_with_its_own_message() {
    let previous = Sample {
        queue_failed_total: Some(10),
        ..sample(NOW - 60)
    };
    let firings = evaluate("shop", &rules(), &sample(NOW), Some(&previous), NOW, |_| {
        None
    });
    assert_eq!(
        kinds(&firings),
        [
            RuleKind::ErrorRate,
            RuleKind::P95,
            RuleKind::QueuePending,
            RuleKind::QueueFailures
        ]
    );
    let messages: Vec<&str> = firings
        .iter()
        .map(|firing| firing.message.as_str())
        .collect();
    assert_eq!(
        messages,
        [
            "shop: 5xx rate 10.00% (threshold 5%)",
            "shop: p95 latency 150.0 ms (threshold 100 ms)",
            "shop: 12 pending jobs (threshold 10)",
            "shop: 5 queue failures since the previous poll (threshold 5)",
        ]
    );
}

#[test]
fn values_below_their_threshold_or_disabled_rules_stay_quiet() {
    let quiet = Sample {
        error_rate_percent: Some(4.9),
        p95_ms: Some(99.9),
        queue_pending: Some(9),
        ..sample(NOW)
    };
    let previous = Sample {
        queue_failed_total: Some(11),
        ..sample(NOW - 60)
    };
    assert!(evaluate("shop", &rules(), &quiet, Some(&previous), NOW, |_| None).is_empty());
    assert!(
        evaluate(
            "shop",
            &AlertRules::default(),
            &sample(NOW),
            None,
            NOW,
            |_| None
        )
        .is_empty()
    );
}

#[test]
fn the_error_rate_needs_enough_requests() {
    let few = Sample {
        window_requests: Some(19),
        error_rate_percent: Some(100.0),
        ..sample(NOW)
    };
    let rules = AlertRules {
        error_rate_percent: Some(5.0),
        ..AlertRules::default()
    };
    assert!(evaluate("shop", &rules, &few, None, NOW, |_| None).is_empty());
    let enough = Sample {
        window_requests: Some(20),
        ..few
    };
    assert_eq!(
        kinds(&evaluate("shop", &rules, &enough, None, NOW, |_| None)),
        [RuleKind::ErrorRate]
    );
}

#[test]
fn a_counter_reset_or_a_missing_previous_sample_never_counts_as_failures() {
    let rules = AlertRules {
        queue_failures: Some(1),
        ..AlertRules::default()
    };
    let restarted = Sample {
        queue_failed_total: Some(2),
        ..sample(NOW)
    };
    let before = Sample {
        queue_failed_total: Some(500),
        ..sample(NOW - 60)
    };
    assert!(evaluate("shop", &rules, &restarted, Some(&before), NOW, |_| None).is_empty());
    assert!(evaluate("shop", &rules, &sample(NOW), None, NOW, |_| None).is_empty());
}

#[test]
fn an_unreachable_project_fires_only_its_own_rule() {
    let down = Sample {
        reachable: false,
        requests_per_second: None,
        window_requests: None,
        error_rate_percent: None,
        p95_ms: None,
        queue_pending: None,
        queue_failed_total: None,
        at: NOW,
    };
    let firings = evaluate(
        "shop",
        &rules(),
        &down,
        Some(&sample(NOW - 60)),
        NOW,
        |_| None,
    );
    assert_eq!(kinds(&firings), [RuleKind::Unreachable]);
    assert_eq!(firings[0].message, "shop could not be read");
    let silent = AlertRules {
        unreachable: false,
        ..rules()
    };
    assert!(evaluate("shop", &silent, &down, None, NOW, |_| None).is_empty());
}

#[test]
fn a_rule_fires_again_only_after_its_cool_down() {
    let rules = AlertRules {
        p95_ms: Some(100.0),
        cooldown_minutes: 15,
        ..AlertRules::default()
    };
    let fired = |seconds_ago: u64| {
        evaluate("shop", &rules, &sample(NOW), None, NOW, move |rule| {
            (rule == RuleKind::P95).then_some(NOW - seconds_ago)
        })
        .len()
    };
    assert_eq!(fired(60), 0);
    assert_eq!(fired(899), 0);
    assert_eq!(fired(900), 1);
    // Another rule's history does not delay this one.
    let other = evaluate("shop", &rules, &sample(NOW), None, NOW, |rule| {
        (rule == RuleKind::Unreachable).then_some(NOW)
    });
    assert_eq!(kinds(&other), [RuleKind::P95]);
}

#[test]
fn thresholds_are_range_checked() {
    assert!(AlertRules::default().valid());
    assert!(rules().valid());
    for invalid in [
        AlertRules {
            error_rate_percent: Some(0.0),
            ..rules()
        },
        AlertRules {
            error_rate_percent: Some(100.1),
            ..rules()
        },
        AlertRules {
            error_rate_percent: Some(f64::NAN),
            ..rules()
        },
        AlertRules {
            p95_ms: Some(-1.0),
            ..rules()
        },
        AlertRules {
            p95_ms: Some(600_001.0),
            ..rules()
        },
        AlertRules {
            queue_pending: Some(0),
            ..rules()
        },
        AlertRules {
            queue_failures: Some(1_000_000_001),
            ..rules()
        },
        AlertRules {
            cooldown_minutes: 0,
            ..rules()
        },
        AlertRules {
            cooldown_minutes: 1_441,
            ..rules()
        },
    ] {
        assert!(!invalid.valid(), "{invalid:?}");
    }
    assert!(
        AlertRules {
            error_rate_percent: Some(100.0),
            cooldown_minutes: 1_440,
            ..rules()
        }
        .valid()
    );
}
