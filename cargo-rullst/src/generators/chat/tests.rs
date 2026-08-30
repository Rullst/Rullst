#![allow(clippy::expect_used)]

use super::*;

#[test]
fn emitted_rust_is_parseable_and_never_discards_persistence_errors() {
    for backend in [ProjectOrmBackend::Sqlx, ProjectOrmBackend::Turso] {
        let model = render_chat_models(backend);
        let service = render_chat_service(backend);
        let migration = render_chat_migration("m20260829000000_chat", backend);
        syn::parse_file(model).expect("chat model template must parse");
        syn::parse_file(service).expect("chat service template must parse");
        syn::parse_file(&migration).expect("chat migration template must parse");
        assert!(!service.contains("let _ ="));
        assert!(service.contains("InvalidHistoryRole"));
        assert!(service.contains("send_lock"));
        assert!(migration.contains("chat_sessions"));
        assert!(migration.contains("chat_messages"));
        assert!(
            migration.contains("DROP TABLE chat_messages")
                || migration.contains("drop_if_exists(\"chat_messages\")")
        );
    }
}

#[test]
fn required_features_are_added_once_to_supported_dependency_forms() {
    for manifest in [
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nrullst = \"12\"\n",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nrullst = { version = \"12\", features = [\"orm\"] }\n",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies.rullst]\nversion = \"12\"\nfeatures = [\"orm\"]\n",
    ] {
        let updated = ensure_rullst_features(manifest, &["orm", "ai"])
            .expect("supported dependency declaration");
        let parsed: toml::Value = toml::from_str(&updated).expect("updated manifest");
        let features = parsed["dependencies"]["rullst"]["features"]
            .as_array()
            .expect("feature array");
        assert_eq!(
            features
                .iter()
                .filter(|feature| feature.as_str() == Some("orm"))
                .count(),
            1
        );
        assert_eq!(
            features
                .iter()
                .filter(|feature| feature.as_str() == Some("ai"))
                .count(),
            1
        );
    }
}

#[test]
fn malformed_or_missing_rullst_dependency_fails_closed() {
    assert!(ensure_rullst_features("not toml", &["ai"]).is_err());
    assert!(
        ensure_rullst_features(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nserde = \"1\"\n",
            &["ai"],
        )
        .is_err()
    );
}
