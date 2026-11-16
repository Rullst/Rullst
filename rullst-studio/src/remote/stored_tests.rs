//! The project list's file format across Studio versions: unknown fields
//! kept and bounded, known fields still validated, later formats refused
//! (`TM-STUDIO-3`, `REMOTE-03`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::alerts::AlertRules;
use super::crypto::SecretKey;
use super::notify::NotificationSettings;
use super::projects::{ProjectError, ProjectStore, WebhookChange, webhook_context};
use super::proxy::{ProxyMode, ProxyPlan, ProxySettings, Unusable};
use super::server_test::{TOKEN, TempDir};
use serde::Deserialize;
use serde_json::{Value, json};

/// The project list of 13.0.0-alpha.5, which refuses unknown fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Alpha5File {
    version: u32,
    projects: Vec<Alpha5Project>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Alpha5Project {
    name: String,
    base_url: String,
    token: String,
    #[serde(default)]
    watch: bool,
    #[serde(default)]
    rules: Option<Alpha5Rules>,
    #[serde(default)]
    webhook: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct Alpha5Rules {
    #[serde(default)]
    unreachable: bool,
    #[serde(default)]
    error_rate_percent: Option<f64>,
    #[serde(default)]
    p95_ms: Option<f64>,
    #[serde(default)]
    queue_pending: Option<u64>,
    #[serde(default)]
    queue_failures: Option<u64>,
    #[serde(default)]
    cooldown_minutes: Option<u32>,
}

pub(super) fn read_json(store: &ProjectStore) -> Value {
    serde_json::from_slice(&std::fs::read(&store.paths().projects).unwrap()).unwrap()
}

pub(super) fn write_json(store: &ProjectStore, value: &Value) {
    std::fs::write(&store.paths().projects, serde_json::to_vec(value).unwrap()).unwrap();
}

fn readable_by_alpha5(store: &ProjectStore) -> bool {
    serde_json::from_slice::<Alpha5File>(&std::fs::read(&store.paths().projects).unwrap()).is_ok()
}

/// A store with one project, `shop`, and its sealed token.
pub(super) fn store_with_shop(label: &str) -> (TempDir, ProjectStore, String) {
    let directory = TempDir::new(label);
    let store = ProjectStore::new(directory.paths());
    store.add("shop", "https://shop.example", TOKEN).unwrap();
    let token = read_json(&store)["projects"][0]["token"]
        .as_str()
        .unwrap()
        .to_string();
    (directory, store, token)
}

#[test]
fn an_alpha5_file_loads_and_stays_readable_by_alpha5_until_a_new_setting_is_saved() {
    let (_directory, store, token) = store_with_shop("alpha5-file");
    let key = SecretKey::load(&store.paths().key).unwrap().unwrap();
    let webhook = key
        .seal(
            "https://hooks.example/a",
            &webhook_context("shop", "https://shop.example/"),
        )
        .unwrap();
    // As 13.0.0-alpha.5 writes it: unset thresholds are `null`.
    write_json(
        &store,
        &json!({
            "version": 1,
            "projects": [{
                "name": "shop",
                "base_url": "https://shop.example/",
                "token": token,
                "watch": true,
                "rules": {
                    "unreachable": true,
                    "error_rate_percent": 5.0,
                    "p95_ms": 800,
                    "queue_pending": null,
                    "queue_failures": 3,
                    "cooldown_minutes": 30
                },
                "webhook": webhook
            }]
        }),
    );
    let project = store.load().unwrap().remove(0);
    assert_eq!(project.token.unwrap().expose(), TOKEN);
    assert_eq!(project.webhook.unwrap().expose(), "https://hooks.example/a");
    assert!(project.watch);
    let expected = AlertRules {
        unreachable: true,
        error_rate_percent: Some(5.0),
        p95_ms: Some(800.0),
        queue_failures: Some(3),
        cooldown_minutes: 30,
        ..AlertRules::default()
    };
    assert_eq!(project.rules, expected);
    assert_eq!(store.settings().unwrap(), Default::default());

    // Saving what alpha.5 knows keeps the file readable by alpha.5.
    store
        .update_settings("shop", true, expected, WebhookChange::Keep)
        .unwrap();
    store.add("blog", "https://blog.example", TOKEN).unwrap();
    assert!(readable_by_alpha5(&store));

    // A p99 rule is a new field: alpha.5 refuses the file from then on.
    let p99 = AlertRules {
        p99_ms: Some(900.0),
        ..expected
    };
    store
        .update_settings("shop", true, p99, WebhookChange::Keep)
        .unwrap();
    assert!(!readable_by_alpha5(&store));
    assert_eq!(store.load().unwrap()[0].rules, p99);
}

/// A file with fields of a newer Studio in every object.
pub(super) fn file_with_unknown_fields(token: &str) -> Value {
    json!({
        "version": 1,
        "layout": {"columns": [2, 3], "dense": true},
        "projects": [{
            "name": "shop",
            "base_url": "https://shop.example/",
            "token": token,
            "labels": ["eu", "prod"],
            "rules": {"p95_ms": 500.0, "p999_ms": 2000.5, "cooldown_minutes": 20}
        }],
        "settings": {
            "theme": "dark",
            "notifications": {"enabled": true, "quiet_minutes": 5, "sound": "chime"},
            "proxy": {"mode": "direct", "pac_url": "https://wpad.example/proxy.pac"}
        }
    })
}

#[test]
fn unknown_fields_load_and_every_rewrite_keeps_them() {
    let (_directory, store, token) = store_with_shop("unknown-fields");
    write_json(&store, &file_with_unknown_fields(&token));
    let project = store.load().unwrap().remove(0);
    assert_eq!(project.token.unwrap().expose(), TOKEN);
    assert_eq!(project.rules.p95_ms, Some(500.0));
    assert_eq!(project.rules.cooldown_minutes, 20);
    let settings = store.settings().unwrap();
    assert_eq!(
        settings.notifications,
        NotificationSettings {
            enabled: true,
            quiet_minutes: 5
        }
    );
    assert_eq!(settings.proxy, ProxySettings::default());

    store.add("blog", "https://blog.example", TOKEN).unwrap();
    let rules = AlertRules {
        p95_ms: Some(700.0),
        ..AlertRules::default()
    };
    store
        .update_settings("shop", true, rules, WebhookChange::Keep)
        .unwrap();
    store
        .update_notifications(NotificationSettings {
            enabled: false,
            quiet_minutes: 7,
        })
        .unwrap();
    store
        .update_proxy(ProxySettings {
            mode: ProxyMode::Environment,
            ..ProxySettings::default()
        })
        .unwrap();
    store.remove("blog").unwrap();

    let written = read_json(&store);
    assert_eq!(written["layout"], json!({"columns": [2, 3], "dense": true}));
    let shop = &written["projects"][0];
    assert_eq!(shop["labels"], json!(["eu", "prod"]));
    assert_eq!(shop["rules"]["p999_ms"], json!(2000.5));
    assert_eq!(shop["rules"]["p95_ms"], json!(700.0));
    assert_eq!(shop["rules"]["cooldown_minutes"], json!(15));
    assert_eq!(shop["watch"], json!(true));
    assert_eq!(written["settings"]["theme"], json!("dark"));
    assert_eq!(
        written["settings"]["notifications"],
        json!({"enabled": false, "quiet_minutes": 7, "sound": "chime"})
    );
    assert_eq!(
        written["settings"]["proxy"],
        json!({"mode": "environment", "pac_url": "https://wpad.example/proxy.pac"})
    );
    assert_eq!(written["projects"].as_array().unwrap().len(), 1);
    // Fields of the removed project leave with it; the rest still loads.
    assert_eq!(store.load().unwrap().len(), 1);
}

#[test]
fn settings_that_only_carry_unknown_fields_are_still_written() {
    let (_directory, store, token) = store_with_shop("settings-extra");
    let mut file = file_with_unknown_fields(&token);
    file["settings"] = json!({"theme": "dark"});
    write_json(&store, &file);
    assert_eq!(store.settings().unwrap(), Default::default());
    store.add("blog", "https://blog.example", TOKEN).unwrap();
    let settings = &read_json(&store)["settings"];
    assert_eq!(settings["theme"], json!("dark"));
    assert_eq!(settings["proxy"], json!({"mode": "direct"}));
}

#[test]
fn a_later_format_is_refused_with_the_studio_that_reads_it() {
    let (_directory, store, _token) = store_with_shop("newer-format");
    let own = env!("CARGO_PKG_VERSION");
    // A later format may change anything else, such as `projects`.
    let newer = json!({"version": 2, "min_studio_version": "14.1.0", "projects": {}});
    write_json(&store, &newer);
    let error = store.load().unwrap_err();
    assert_eq!(
        error,
        ProjectError::NewerFormat {
            format: 2,
            studio: Some(14)
        }
    );
    assert_eq!(
        error.to_string(),
        format!(
            "the project list uses file format 2, which needs Rullst Studio 14 or later; this Studio ({own}) reads format 1"
        )
    );
    // Nothing overwrites a file of a later format.
    let before = std::fs::read(&store.paths().projects).unwrap();
    assert_eq!(
        store
            .add("blog", "https://blog.example", TOKEN)
            .unwrap_err(),
        error
    );
    assert_eq!(
        store.update_notifications(NotificationSettings::default()),
        Err(error)
    );
    assert_eq!(std::fs::read(&store.paths().projects).unwrap(), before);

    let longest = format!("15.0.0-{}", "a".repeat(57));
    assert_eq!(longest.len(), 64);
    for (version, min_studio_version, studio) in [
        (json!(3), json!(null), None),
        (json!(3), json!(15), None),
        (json!(3), json!("next"), None),
        (json!(3), json!(longest), Some(15)),
        (json!(3), json!(format!("{longest}a")), None),
    ] {
        write_json(
            &store,
            &json!({"version": version, "min_studio_version": min_studio_version}),
        );
        assert_eq!(
            store.load().unwrap_err(),
            ProjectError::NewerFormat { format: 3, studio },
            "{min_studio_version}"
        );
    }
    write_json(&store, &json!({"version": 3}));
    assert_eq!(
        store.load().unwrap_err().to_string(),
        format!(
            "the project list uses file format 3, written by a newer Rullst Studio; this Studio ({own}) reads format 1"
        )
    );
    for malformed in [
        json!({"version": 0, "projects": []}),
        json!({"version": "2"}),
    ] {
        write_json(&store, &malformed);
        assert_eq!(
            store.load().unwrap_err(),
            ProjectError::InvalidFile,
            "{malformed}"
        );
    }
}

#[test]
fn a_proxy_this_studio_cannot_use_loads_and_blocks_every_connection() {
    let (_directory, store, token) = store_with_shop("unusable-proxy");
    let base = file_with_unknown_fields(&token);
    for (proxy, unusable) in [
        (
            json!({"mode": "explicit", "url": "socks5://proxy.internal:1080"}),
            Unusable::Socks("The saved proxy URL"),
        ),
        (
            json!({"mode": "explicit"}),
            Unusable::Malformed("The saved proxy URL"),
        ),
        (
            json!({"mode": "explicit", "url": "http://p:1", "no_proxy": "a".repeat(4_097)}),
            Unusable::LongExceptions("The saved exception list"),
        ),
    ] {
        write_json(&store, &edited(&base, "/settings/proxy", proxy.clone()));
        let settings = store.settings().unwrap().proxy;
        assert!(!settings.valid(), "{proxy}");
        assert_eq!(
            ProxyPlan::from_settings(&settings),
            ProxyPlan::Invalid(unusable)
        );
        assert_eq!(store.load().unwrap().len(), 1);
    }
}

pub(super) fn edited(file: &Value, pointer: &str, value: Value) -> Value {
    let mut file = file.clone();
    match file.pointer_mut(pointer) {
        Some(slot) => *slot = value,
        None => {
            let (parent, name) = pointer.rsplit_once('/').unwrap();
            file.pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(name.to_string(), value);
        }
    }
    file
}

#[test]
fn a_proxy_mode_of_a_newer_studio_is_kept_and_blocks_every_connection() {
    let (_directory, store, token) = store_with_shop("newer-proxy-mode");
    let file = edited(
        &file_with_unknown_fields(&token),
        "/settings/proxy/mode",
        json!("pac"),
    );
    write_json(&store, &file);
    let proxy = store.settings().unwrap().proxy;
    assert_eq!(proxy.mode, ProxyMode::Newer("pac".to_string()));
    // The Settings page cannot save a mode it does not know.
    assert!(!proxy.valid());
    assert_eq!(
        ProxyPlan::from_settings(&proxy),
        ProxyPlan::Invalid(Unusable::NewerMode)
    );
    store
        .update_notifications(NotificationSettings::default())
        .unwrap();
    assert_eq!(read_json(&store)["settings"]["proxy"]["mode"], json!("pac"));
    store.update_proxy(ProxySettings::default()).unwrap();
    assert_eq!(
        read_json(&store)["settings"]["proxy"],
        json!({"mode": "direct", "pac_url": "https://wpad.example/proxy.pac"})
    );
}
