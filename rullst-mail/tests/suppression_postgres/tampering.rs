use super::support::*;

/// Quota columns and row counts written around the adapter fail closed even
/// when the namespace binding still matches the configured quotas.
pub async fn run(url: &str) {
    let namespace = unique();
    let limits = PostgresSuppressionConfig::new(&namespace, 2, 2).unwrap();
    let store = PostgresSuppressionStore::initialize(url, key(), limits)
        .await
        .unwrap();
    let raw = sqlx::PgPool::connect(url).await.unwrap();
    for (tamper, restore) in [
        (
            "UPDATE rullst_mail_pg_suppression_control SET max_recipients = 3 WHERE namespace = $1",
            "UPDATE rullst_mail_pg_suppression_control SET max_recipients = 2 WHERE namespace = $1",
        ),
        (
            "UPDATE rullst_mail_pg_suppression_control SET max_events = 3 WHERE namespace = $1",
            "UPDATE rullst_mail_pg_suppression_control SET max_events = 2 WHERE namespace = $1",
        ),
    ] {
        sqlx::query(tamper)
            .bind(&namespace)
            .execute(&raw)
            .await
            .unwrap();
        assert_eq!(
            store.snapshot().await.unwrap_err(),
            SuppressionError::InvalidConfiguration("namespace binding")
        );
        sqlx::query(restore)
            .bind(&namespace)
            .execute(&raw)
            .await
            .unwrap();
        assert_eq!(store.snapshot().await.unwrap().events(), 0);
    }
    let time = i64::try_from(now() - 60).unwrap();
    for (insert, delete) in [
        (
            "INSERT INTO rullst_mail_pg_suppression_events
             (namespace,event_tag,fingerprint,observed_at) VALUES ($1,$2,$2,$3)",
            "DELETE FROM rullst_mail_pg_suppression_events WHERE namespace = $1",
        ),
        (
            "INSERT INTO rullst_mail_pg_suppression_recipients
             (namespace,recipient_tag,reason,provider,first_seen_at,last_seen_at)
             VALUES ($1,$2,1,'fixture',$3,$3)",
            "DELETE FROM rullst_mail_pg_suppression_recipients WHERE namespace = $1",
        ),
    ] {
        for index in 0..3_u8 {
            sqlx::query(insert)
                .bind(&namespace)
                .bind(vec![index; 32])
                .bind(time)
                .execute(&raw)
                .await
                .unwrap();
        }
        // Three rows exceed one quota of two while the other count is zero.
        assert_eq!(
            store.snapshot().await.unwrap_err(),
            SuppressionError::CorruptStorage("state quota")
        );
        sqlx::query(delete)
            .bind(&namespace)
            .execute(&raw)
            .await
            .unwrap();
        assert_eq!(store.snapshot().await.unwrap().recipients(), 0);
    }
    raw.close().await;
    store.close().await;
}
