//! Desktop notification debouncing with a fixed clock (`TM-STUDIO-3`,
//! `REMOTE-12`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::notify::{MAX_QUEUED, Notification, NotificationSettings, Notifier};

const NOW: u64 = 1_800_000_000;
/// Ten minutes, the default quiet period.
const QUIET: u64 = 600;

fn enabled() -> NotificationSettings {
    NotificationSettings {
        enabled: true,
        ..NotificationSettings::default()
    }
}

fn bodies(notifier: &Notifier) -> Vec<String> {
    let (_, pending) = notifier.since(Some(notifier.session()), Some(0));
    pending
        .into_iter()
        .map(|notification| notification.body)
        .collect()
}

#[test]
fn notifications_are_off_until_enabled_and_wait_ten_minutes_by_default() {
    let settings = NotificationSettings::default();
    assert!(!settings.enabled);
    assert_eq!(settings.quiet_minutes, 10);
    let notifier = Notifier::new("s1".to_string());
    notifier.offer(&settings, "shop", "shop could not be read", NOW);
    notifier.flush(&settings, NOW + QUIET);
    assert_eq!(notifier.since(Some("s1"), Some(0)), (0, Vec::new()));
}

#[test]
fn one_notification_per_project_per_quiet_period() {
    let notifier = Notifier::new("s1".to_string());
    let settings = enabled();
    notifier.offer(&settings, "shop", "first", NOW);
    notifier.offer(&settings, "shop", "held", NOW + QUIET - 1);
    // Another project has its own quiet period.
    notifier.offer(&settings, "blog", "blog alert", NOW + 1);
    notifier.offer(&settings, "shop", "second", NOW + QUIET);
    notifier.offer(&settings, "shop", "third", NOW + QUIET + 1);
    assert_eq!(
        bodies(&notifier),
        [
            "first",
            "blog alert",
            "second (and 1 more since the previous notification)"
        ]
    );
    let (cursor, pending) = notifier.since(Some("s1"), Some(0));
    assert_eq!(cursor, 3);
    assert_eq!(
        pending[0],
        Notification {
            id: 1,
            title: "Rullst Studio · shop".to_string(),
            body: "first".to_string()
        }
    );
    assert_eq!(pending[1].title, "Rullst Studio · blog");
}

#[test]
fn held_alerts_are_summarized_once_the_quiet_period_ends() {
    let notifier = Notifier::new("s1".to_string());
    let settings = NotificationSettings {
        enabled: true,
        quiet_minutes: 1,
    };
    notifier.offer(&settings, "shop", "first", NOW);
    notifier.offer(&settings, "shop", "a", NOW + 10);
    notifier.offer(&settings, "shop", "b", NOW + 20);
    notifier.offer(&settings, "blog", "only", NOW);
    notifier.flush(&settings, NOW + 59);
    assert_eq!(bodies(&notifier).len(), 2);
    notifier.flush(&settings, NOW + 60);
    // The summary restarts the quiet period, and nothing is held any more.
    notifier.offer(&settings, "shop", "held again", NOW + 119);
    notifier.flush(&settings, NOW + 200);
    assert_eq!(
        bodies(&notifier),
        [
            "first",
            "only",
            "2 more alerts since the previous notification; see the Alerts page",
            "1 more alerts since the previous notification; see the Alerts page",
        ]
    );
    // Disabled settings neither show nor summarize.
    let off = NotificationSettings::default();
    notifier.offer(&settings, "blog", "held", NOW + 30);
    notifier.flush(&off, NOW + 1_000);
    notifier.offer(&off, "news", "off", NOW + 1_000);
    assert_eq!(bodies(&notifier).len(), 4);
}

#[test]
fn summaries_of_several_projects_come_in_name_order() {
    let notifier = Notifier::new("s1".to_string());
    let settings = enabled();
    for project in ["shop", "blog", "api"] {
        notifier.offer(&settings, project, "first", NOW);
        notifier.offer(&settings, project, "held", NOW + 1);
    }
    notifier.flush(&settings, NOW + QUIET);
    let (_, pending) = notifier.since(Some("s1"), Some(3));
    let titles: Vec<&str> = pending.iter().map(|item| item.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Rullst Studio · api",
            "Rullst Studio · blog",
            "Rullst Studio · shop"
        ]
    );
}

#[test]
fn pages_read_after_their_cursor_of_this_session_only() {
    let notifier = Notifier::new("s1".to_string());
    let settings = enabled();
    for (index, project) in ["a", "b", "c"].iter().enumerate() {
        notifier.offer(&settings, project, "alert", NOW + index as u64);
    }
    let ids = |pending: Vec<Notification>| -> Vec<u64> {
        pending
            .into_iter()
            .map(|notification| notification.id)
            .collect()
    };
    let (cursor, pending) = notifier.since(Some("s1"), Some(1));
    assert_eq!((cursor, ids(pending)), (3, vec![2, 3]));
    assert_eq!(notifier.since(Some("s1"), Some(3)), (3, Vec::new()));
    // No cursor, or one of an earlier Studio process: start at the newest.
    assert_eq!(notifier.since(None, None), (3, Vec::new()));
    assert_eq!(notifier.since(Some("s1"), None), (3, Vec::new()));
    assert_eq!(notifier.since(Some("old"), Some(0)), (3, Vec::new()));
    assert_eq!(notifier.session(), "s1");
}

#[test]
fn the_queue_keeps_the_newest_notifications() {
    let notifier = Notifier::new("s1".to_string());
    let settings = enabled();
    for index in 0..=MAX_QUEUED {
        notifier.offer(&settings, &format!("p{index}"), "alert", NOW);
    }
    let (cursor, pending) = notifier.since(Some("s1"), Some(0));
    assert_eq!(cursor, 33);
    assert_eq!(pending.len(), 32);
    assert_eq!(pending[0].id, 2);
}

#[test]
fn a_removed_project_starts_over_and_settings_are_range_checked() {
    let notifier = Notifier::new("s1".to_string());
    let settings = enabled();
    notifier.offer(&settings, "shop", "first", NOW);
    notifier.offer(&settings, "shop", "held", NOW + 1);
    notifier.forget("shop");
    notifier.offer(&settings, "shop", "again", NOW + 2);
    notifier.flush(&settings, NOW + QUIET + 2);
    assert_eq!(bodies(&notifier), ["first", "again"]);
    for (quiet_minutes, valid) in [(0, false), (1, true), (1_440, true), (1_441, false)] {
        let settings = NotificationSettings {
            enabled: true,
            quiet_minutes,
        };
        assert_eq!(settings.valid(), valid, "{quiet_minutes}");
    }
    let read: NotificationSettings = serde_json::from_str(r#"{"enabled": true}"#).unwrap();
    assert_eq!(read.quiet_minutes, 10);
}
