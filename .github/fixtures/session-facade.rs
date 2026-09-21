use rullst::auth::recovery::{RecoveryError, RecoverySecrets, SessionLabel, SqlRecoveryStore};

fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}

#[test]
fn facade_sessions_share_authoritative_logout_without_enabling_mail() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session facade.db");
    let encoded: String = path
        .to_str()
        .unwrap()
        .bytes()
        .map(|byte| format!("%{byte:02X}"))
        .collect();
    let url = format!("sqlite:{encoded}?mode=rwc");
    let runtime = rullst::async_runtime::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let a = SqlRecoveryStore::connect(&url, keys()).await.unwrap();
        a.migrate().await.unwrap();
        let b = SqlRecoveryStore::connect(&url, keys()).await.unwrap();
        let password = format!(
            "Fixture_{}",
            directory.path().file_name().unwrap().to_str().unwrap()
        );
        a.register_account("owner", "owner@example.com", &password, 1000)
            .await
            .unwrap();
        let owner = a
            .authenticate("owner@example.com", password)
            .await
            .unwrap()
            .unwrap();
        let current = a
            .create_session_with_label(
                &owner,
                1000,
                600,
                SessionLabel::new("Current browser").unwrap(),
            )
            .await
            .unwrap();
        let sibling = a
            .create_session_with_label(
                &owner,
                1000,
                600,
                SessionLabel::new("Other browser").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            b.active_sessions(current.expose(), 1001)
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            b.revoke_other_sessions(current.expose(), 1001)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            a.verify_session(sibling.expose(), 1001).await.unwrap(),
            None
        );
        assert_eq!(
            a.verify_session(current.expose(), 1001)
                .await
                .unwrap()
                .as_deref(),
            Some("owner")
        );
        assert!(matches!(
            a.create_session(&owner, 1001, 60).await,
            Err(RecoveryError::InvalidAction)
        ));
        assert_eq!(a.purge_expired_sessions(1600, 100).await.unwrap(), 1);
        assert_eq!(
            b.verify_session(current.expose(), 1600).await.unwrap(),
            None
        );
        a.close().await;
        b.close().await;
    });
}
