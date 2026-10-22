use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

const UPLOAD: &str = "14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5";
const PLAY: &str = "aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99:aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99";
const PLAY_UPPER: &str = "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99";

#[test]
fn fingerprints_are_32_colon_separated_hex_bytes() {
    let app = AndroidApp::new("com.acme.notes", [UPLOAD, PLAY]).unwrap();
    assert_eq!(app.package_name(), "com.acme.notes");
    assert_eq!(app.sha256_cert_fingerprints(), [UPLOAD, PLAY_UPPER]);

    let invalid = [
        // 31 bytes, 33 bytes, a three-digit byte, a non-hex digit, other separators.
        UPLOAD.rsplit_once(':').unwrap().0.to_owned(),
        format!("{UPLOAD}:00"),
        UPLOAD.replacen("14", "141", 1),
        UPLOAD.replacen("14", "1G", 1),
        UPLOAD.replace(':', "-"),
        UPLOAD.replace(':', ""),
        String::new(),
    ];
    for fingerprint in invalid {
        assert_eq!(
            AndroidApp::new("com.acme.notes", [PLAY, fingerprint.as_str()]),
            Err(AssetLinksError::InvalidFingerprint {
                package: "com.acme.notes".to_owned(),
                position: 2,
            }),
            "{fingerprint}"
        );
    }
    assert_eq!(
        AndroidApp::new("com.acme.notes", Vec::<String>::new()),
        Err(AssetLinksError::MissingFingerprint(
            "com.acme.notes".to_owned()
        ))
    );
    assert_eq!(
        AssetLinksError::InvalidFingerprint {
            package: "com.acme.notes".to_owned(),
            position: 2,
        }
        .to_string(),
        "SHA-256 certificate fingerprint 2 of `com.acme.notes` is not 32 colon-separated hexadecimal bytes"
    );
}

#[test]
fn package_names_are_android_application_ids() {
    for valid in ["com.acme.notes", "Com.Acme_2.app", "a.b"] {
        assert!(AndroidApp::new(valid, [UPLOAD]).is_ok(), "{valid}");
    }
    let long = format!("com.{}", "a".repeat(252));
    assert!(AndroidApp::new(long.as_str(), [UPLOAD]).is_err());
    assert!(AndroidApp::new(&long[..255], [UPLOAD]).is_ok());
    for invalid in [
        "notes",
        "com..notes",
        "com.1acme",
        "com.acme-notes",
        "com.acme.",
        ".com.acme",
        "com.ac me",
    ] {
        assert_eq!(
            AndroidApp::new(invalid, [UPLOAD]),
            Err(AssetLinksError::InvalidPackageName(invalid.to_owned())),
            "{invalid}"
        );
    }
}

#[test]
fn the_statement_list_is_exact() {
    let links = AssetLinks::new()
        .with_app(AndroidApp::new("com.acme.notes", [UPLOAD, PLAY]).unwrap())
        .with_app(AndroidApp::new("com.acme.beta", [UPLOAD]).unwrap());
    let expected = format!(
        "[{{\"relation\":[\"delegate_permission/common.handle_all_urls\"],\"target\":{{\"namespace\":\"android_app\",\"package_name\":\"com.acme.notes\",\"sha256_cert_fingerprints\":[\"{UPLOAD}\",\"{PLAY_UPPER}\"]}}}},{{\"relation\":[\"delegate_permission/common.handle_all_urls\"],\"target\":{{\"namespace\":\"android_app\",\"package_name\":\"com.acme.beta\",\"sha256_cert_fingerprints\":[\"{UPLOAD}\"]}}}}]"
    );
    assert_eq!(links.to_json(), expected);
    let parsed: serde_json::Value = serde_json::from_str(&links.to_json()).unwrap();
    assert_eq!(parsed[1]["target"]["package_name"], "com.acme.beta");
    assert_eq!(AssetLinks::new().to_json(), "[]");
    assert!(AssetLinks::new().is_empty());
    assert!(!links.is_empty());
    assert_eq!(links.apps().len(), 2);
}

#[test]
fn the_environment_format_lists_apps_and_fingerprints() {
    let links = AssetLinks::parse(&format!(
        " com.acme.notes = {UPLOAD} , {PLAY} ; ; com.acme.beta={UPLOAD};"
    ))
    .unwrap();
    assert_eq!(
        links,
        AssetLinks::new()
            .with_app(AndroidApp::new("com.acme.notes", [UPLOAD, PLAY]).unwrap())
            .with_app(AndroidApp::new("com.acme.beta", [UPLOAD]).unwrap())
    );
    assert_eq!(
        AssetLinks::parse(&format!("com.acme.notes={UPLOAD};com.acme.beta")),
        Err(AssetLinksError::Configuration(
            "entry 2 of RULLST_ANDROID_ASSET_LINKS has no `=` between package and fingerprints"
                .to_owned()
        ))
    );
    assert_eq!(
        AssetLinks::parse("com.acme.notes=,"),
        Err(AssetLinksError::MissingFingerprint(
            "com.acme.notes".to_owned()
        ))
    );
    assert!(AssetLinks::parse("").unwrap().is_empty());
}

#[test]
fn rullst_toml_tables_are_read_and_errors_name_only_a_position() {
    let content = format!(
        "[app]\nport = 3000\n\n[[android.asset_links]]\npackage_name = \"com.acme.notes\"\nsha256_cert_fingerprints = [\"{UPLOAD}\"]\n"
    );
    assert_eq!(
        AssetLinks::from_toml(&content).unwrap(),
        AssetLinks::new().with_app(AndroidApp::new("com.acme.notes", [UPLOAD]).unwrap())
    );
    assert!(
        AssetLinks::from_toml("[app]\nport = 3000\n")
            .unwrap()
            .is_empty()
    );
    let secret = "[database]\nurl = \"postgres://user:hunter2@db\"\n[[android.asset_links]]\npackage_name = 7\n";
    let error = AssetLinks::from_toml(secret).unwrap_err().to_string();
    assert_eq!(
        error,
        "invalid Android asset links configuration: Rullst.toml is invalid at line 4, column 16"
    );
    let misspelled =
        "[[android.asset_links]]\npackage_name = \"com.acme.notes\"\nfingerprints = []\n";
    assert!(AssetLinks::from_toml(misspelled).is_err());
    assert_eq!(
        AssetLinks::from_toml(&content.replace("com.acme.notes", "notes")),
        Err(AssetLinksError::InvalidPackageName("notes".to_owned()))
    );
}

#[test]
fn the_project_prefers_its_dotenv_setting_over_rullst_toml() {
    let directory = tempfile::tempdir().unwrap();
    assert!(
        AssetLinks::from_project_in(directory.path())
            .unwrap()
            .is_empty()
    );
    std::fs::write(
        directory.path().join("Rullst.toml"),
        format!("[[android.asset_links]]\npackage_name = \"com.acme.notes\"\nsha256_cert_fingerprints = [\"{UPLOAD}\"]\n"),
    )
    .unwrap();
    let from_toml = AssetLinks::from_project_in(directory.path()).unwrap();
    assert_eq!(from_toml.apps()[0].package_name(), "com.acme.notes");

    // An empty setting does not hide the Rullst.toml tables.
    std::fs::write(
        directory.path().join(".env"),
        "RULLST_ANDROID_ASSET_LINKS=\" \"\n",
    )
    .unwrap();
    assert_eq!(
        AssetLinks::from_project_in(directory.path()).unwrap(),
        from_toml
    );
    std::fs::write(
        directory.path().join(".env"),
        format!("RULLST_ANDROID_ASSET_LINKS=com.acme.beta={PLAY}\n"),
    )
    .unwrap();
    let from_env = AssetLinks::from_project_in(directory.path()).unwrap();
    assert_eq!(from_env.apps()[0].package_name(), "com.acme.beta");
    assert_eq!(from_env.apps()[0].sha256_cert_fingerprints(), [PLAY_UPPER]);

    std::fs::write(directory.path().join(".env"), "not a dotenv line\n").unwrap();
    assert!(matches!(
        AssetLinks::from_project_in(directory.path()),
        Err(AssetLinksError::Configuration(_))
    ));
    std::fs::remove_file(directory.path().join(".env")).unwrap();
    std::fs::remove_file(directory.path().join("Rullst.toml")).unwrap();
    std::fs::create_dir(directory.path().join("Rullst.toml")).unwrap();
    assert!(matches!(
        AssetLinks::from_project_in(directory.path()),
        Err(AssetLinksError::Configuration(message)) if message.starts_with("Rullst.toml could not be read")
    ));
}

#[tokio::test]
async fn the_route_serves_cacheable_json_only_when_an_app_is_configured() {
    let links = AssetLinks::new().with_app(AndroidApp::new("com.acme.notes", [UPLOAD]).unwrap());
    let request = || {
        Request::get("/.well-known/assetlinks.json")
            .body(Body::empty())
            .unwrap()
    };
    let router = links.router();
    let response = router.clone().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "public, max-age=3600"
    );
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert_eq!(body, links.to_json().as_bytes());
    // The router answers the same body again.
    let again = router.oneshot(request()).await.unwrap();
    assert_eq!(to_bytes(again.into_body(), 4096).await.unwrap(), body);

    let empty = AssetLinks::new().router().oneshot(request()).await.unwrap();
    assert_eq!(empty.status(), StatusCode::NOT_FOUND);
}
