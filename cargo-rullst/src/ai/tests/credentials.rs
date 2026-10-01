use super::*;
use std::collections::HashMap;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    move |name| map.get(name).cloned()
}

fn stored(provider: Provider, model: Option<&str>, secret: Option<&str>) -> Stored {
    Stored {
        provider,
        model: model.map(str::to_string),
        secret: secret.map(Secret::new),
    }
}

#[test]
fn environment_variables_take_precedence_over_the_file() {
    let file = stored(Provider::OpenAi, Some("gpt-file"), Some("sk-file"));
    let resolved = resolve(
        None,
        None,
        Some(&file),
        env(&[("OPENAI_API_KEY", "sk-env")]),
    )
    .unwrap();
    assert_eq!(resolved.provider, Some(Provider::OpenAi));
    assert_eq!(resolved.secret.expose(), "sk-env");
    assert_eq!(resolved.source, KeySource::Env("OPENAI_API_KEY"));
    assert_eq!(resolved.model, "gpt-file");
    assert!(!resolved.mock);

    let resolved = resolve(None, None, Some(&file), env(&[])).unwrap();
    assert_eq!(resolved.secret.expose(), "sk-file");
    assert_eq!(resolved.source, KeySource::File);

    // Blank environment values do not mask the stored key.
    let resolved = resolve(None, None, Some(&file), env(&[("OPENAI_API_KEY", "  ")])).unwrap();
    assert_eq!(resolved.source, KeySource::File);
}

#[test]
fn provider_and_model_follow_flag_file_environment_default_order() {
    let file = stored(Provider::Anthropic, Some("claude-file"), Some("ak-file"));
    // A flag selects another provider; the stored key and model are not reused.
    let resolved = resolve(
        Some(Provider::Gemini),
        None,
        Some(&file),
        env(&[("GEMINI_API_KEY", "g-env")]),
    )
    .unwrap();
    assert_eq!(resolved.provider, Some(Provider::Gemini));
    assert_eq!(resolved.model, Provider::Gemini.default_model());
    assert!(resolved.model_is_default);

    let resolved = resolve(
        None,
        Some("claude-flag"),
        Some(&file),
        env(&[("RULLST_AI_MODEL", "claude-env")]),
    )
    .unwrap();
    assert_eq!(resolved.model, "claude-flag");
    let resolved = resolve(
        None,
        None,
        Some(&file),
        env(&[("RULLST_AI_MODEL", "claude-env")]),
    )
    .unwrap();
    assert_eq!(resolved.model, "claude-env");

    // Without a file, the first configured environment provider is used.
    let resolved = resolve(
        None,
        None,
        None,
        env(&[("DEEPSEEK_API_KEY", "d"), ("GEMINI_API_KEY", "g")]),
    )
    .unwrap();
    assert_eq!(resolved.provider, Some(Provider::Gemini));

    assert!(resolve(None, Some("bad model"), None, env(&[])).is_err());
}

#[test]
fn nothing_configured_or_mock_keys_select_the_offline_assistant() {
    let resolved = resolve(None, None, None, env(&[])).unwrap();
    assert_eq!(resolved.provider, None);
    assert!(resolved.mock);

    let resolved = resolve(None, None, None, env(&[("OPENAI_API_KEY", "mock_demo")])).unwrap();
    assert_eq!(resolved.provider, Some(Provider::OpenAi));
    assert!(resolved.mock);

    // An explicit provider without a key is offline too.
    let resolved = resolve(Some(Provider::Anthropic), None, None, env(&[])).unwrap();
    assert!(resolved.mock);
    assert_eq!(resolved.source, KeySource::Missing);

    // Ollama falls back to the local default host, which is live.
    let resolved = resolve(Some(Provider::Ollama), None, None, env(&[])).unwrap();
    assert_eq!(resolved.secret.expose(), DEFAULT_OLLAMA_HOST);
    assert_eq!(resolved.source, KeySource::Default);
    assert!(!resolved.mock);
}

#[test]
fn debug_output_never_contains_the_secret() {
    let resolved = resolve(
        None,
        None,
        None,
        env(&[("OPENAI_API_KEY", "sk-very-secret")]),
    )
    .unwrap();
    let rendered = format!("{resolved:?}");
    assert!(!rendered.contains("very-secret"));
    assert!(rendered.contains("[REDACTED]"));
}

#[test]
fn config_base_ignores_relative_values() {
    let base = |pairs: &[(&str, &str)]| {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), OsString::from(value)))
            .collect();
        config_base(move |name| map.get(name).cloned())
    };
    if cfg!(windows) {
        assert_eq!(
            base(&[("APPDATA", r"C:\Users\dev\AppData\Roaming")]),
            Some(PathBuf::from(r"C:\Users\dev\AppData\Roaming"))
        );
        assert_eq!(base(&[("APPDATA", "relative")]), None);
    } else {
        assert_eq!(
            base(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/dev")]),
            Some(PathBuf::from("/xdg"))
        );
        assert_eq!(
            base(&[("XDG_CONFIG_HOME", "relative"), ("HOME", "/home/dev")]),
            Some(PathBuf::from("/home/dev/.config"))
        );
        assert_eq!(base(&[]), None);
    }
}

#[test]
fn saved_files_round_trip_with_owner_only_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config").join("rullst").join(FILE);
    let original = stored(
        Provider::DeepSeek,
        Some("deepseek-chat"),
        Some("sk-round-trip"),
    );
    save(&path, &original).unwrap();
    let loaded = load(&path).unwrap().unwrap();
    assert_eq!(loaded.stored, original);
    assert!(!loaded.insecure_permissions);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("# Rullst AI credentials"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let parent = fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(parent & 0o777, 0o700);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(&path).unwrap().unwrap().insecure_permissions);
        // Rewriting restores owner-only permissions.
        save(&path, &original).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let ollama = stored(Provider::Ollama, None, Some("http://127.0.0.1:11434"));
    save(&path, &ollama).unwrap();
    assert_eq!(load(&path).unwrap().unwrap().stored, ollama);
    assert!(fs::read_to_string(&path).unwrap().contains("host = "));

    assert!(remove(&path).unwrap());
    assert!(!remove(&path).unwrap());
    assert!(load(&path).unwrap().is_none());
}

#[test]
fn malformed_files_fail_without_echoing_their_contents() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(FILE);
    for body in [
        "version = 1\nprovider = \"openai\"\napi_key = \"two words\"\n",
        "version = 1\nprovider = \"shell\"\napi_key = \"sk-leak-me\"\n",
        "version = 2\nprovider = \"openai\"\n",
        "version = 1\nprovider = \"openai\"\nunknown = \"sk-leak-me\"\n",
        "version = 1\nprovider = \"openai\"\nhost = \"http://x\"\n",
        "not toml sk-leak-me",
    ] {
        fs::write(&path, body).unwrap();
        let error = load(&path).unwrap_err();
        assert!(matches!(error, CredentialError::Invalid(_)));
        assert!(!error.to_string().contains("leak-me"));
    }
    fs::write(&path, "x".repeat(MAX_FILE_BYTES as usize + 1)).unwrap();
    assert!(matches!(load(&path), Err(CredentialError::Invalid(_))));
}

#[cfg(unix)]
#[test]
fn symlinked_credentials_are_refused() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("elsewhere.toml");
    fs::write(&target, "version = 1\nprovider = \"openai\"\n").unwrap();
    let link = directory.path().join(FILE);
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(matches!(load(&link), Err(CredentialError::NotRegular(_))));
    let original = stored(Provider::OpenAi, None, Some("sk-x"));
    assert!(matches!(
        save(&link, &original),
        Err(CredentialError::NotRegular(_))
    ));
    // Disconnect removes the link, never its target.
    assert!(remove(&link).unwrap());
    assert!(target.exists());
}

#[test]
fn locations_inside_the_project_or_a_git_work_tree_are_refused() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().join("app");
    fs::create_dir_all(project.join("sub")).unwrap();
    let inside = project.join("sub/config/rullst").join(FILE);
    assert!(matches!(
        check_location(&inside, Some(&project), true),
        Err(CredentialError::InsideProject(_))
    ));
    assert!(matches!(
        check_location(&inside, Some(&project), false),
        Err(CredentialError::InsideProject(_))
    ));

    let home = directory.path().join("home");
    fs::create_dir_all(home.join(".config")).unwrap();
    let outside = home.join(".config/rullst").join(FILE);
    check_location(&outside, Some(&project), true).unwrap();

    fs::create_dir_all(home.join(".git")).unwrap();
    assert!(matches!(
        check_location(&outside, Some(&project), true),
        Err(CredentialError::InsideGitWorkTree(_))
    ));
    // Reading an existing file from such a location is still allowed.
    check_location(&outside, Some(&project), false).unwrap();
}

#[cfg(unix)]
#[test]
fn a_configuration_directory_linked_into_the_project_is_detected() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().join("app");
    fs::create_dir_all(project.join("secrets")).unwrap();
    let config = directory.path().join("config");
    fs::create_dir_all(&config).unwrap();
    std::os::unix::fs::symlink(project.join("secrets"), config.join("rullst")).unwrap();
    assert!(matches!(
        check_location(&config.join("rullst").join(FILE), Some(&project), true),
        Err(CredentialError::InsideProject(_))
    ));
}
