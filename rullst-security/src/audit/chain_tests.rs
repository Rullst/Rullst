//! Key-diversity, timestamp, sequence and stdout-logger checks of the chain.

use super::*;

const SECRET: &[u8] = b"audit-key-material-with-32-plus-bytes";

/// A record correctly signed for exactly the given sequence and predecessor.
fn signed_record(sequence_id: u64, previous_hash: &str) -> AuditRecord {
    let material = canonical_record_material(
        sequence_id,
        1_700_000_000,
        "actor",
        "create",
        "course:1",
        "{}",
        previous_hash,
    )
    .expect("canonical material");
    AuditRecord {
        sequence_id,
        timestamp: 1_700_000_000,
        actor: "actor".to_string(),
        action: "create".to_string(),
        resource: "course:1".to_string(),
        payload: "{}".to_string(),
        previous_hash: previous_hash.to_string(),
        hash: sign_material(SECRET, &material).expect("signature"),
    }
}

#[test]
fn a_signed_record_with_the_wrong_sequence_number_breaks_the_chain() {
    let record = signed_record(2, GENESIS_HASH);
    assert!(AuditChain::verify_record(SECRET, &record));
    assert!(!AuditChain::verify_sequence(SECRET, &[record]));
    assert!(AuditChain::verify_sequence(
        SECRET,
        &[signed_record(1, GENESIS_HASH)]
    ));
}

#[test]
fn keys_need_eight_distinct_byte_values() {
    let logger: Arc<dyn AuditLogger> = Arc::new(StdoutAuditLogger);
    let eight_distinct = b"01234567".repeat(4);
    assert!(AuditChain::try_new(&eight_distinct, logger.clone()).is_ok());
    let seven_distinct = b"0123456".repeat(5);
    assert!(AuditChain::try_new(&seven_distinct[..32], logger).is_err());
}

#[tokio::test]
async fn records_carry_the_current_unix_time() {
    let chain = AuditChain::try_new(SECRET, Arc::new(StdoutAuditLogger)).expect("chain");
    let now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after the epoch")
            .as_secs()
    };
    let before = now();
    let record = chain
        .record_event("actor", "create", "course:1", "{}")
        .await
        .expect("record");
    assert!(record.timestamp >= before && record.timestamp <= now());
}

const STDOUT_CHILD: &str = "RULLST_AUDIT_STDOUT_CHILD";

#[test]
#[cfg_attr(miri, ignore)]
fn the_stdout_logger_prints_one_line_per_record() {
    let record = signed_record(7, GENESIS_HASH);
    if std::env::var_os(STDOUT_CHILD).is_some() {
        StdoutAuditLogger.log(&record).expect("stdout logging");
        return;
    }
    // Re-run only this test in a child process and read its real stdout.
    let full_name = concat!(
        module_path!(),
        "::the_stdout_logger_prints_one_line_per_record"
    );
    let name = full_name
        .split_once("::")
        .map_or(full_name, |(_, rest)| rest);
    let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(STDOUT_CHILD, "1")
        .output()
        .expect("run the child test");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("[AUDIT LOG #7]").count(), 1, "{stdout}");
    assert!(stdout.contains(&stdout_audit_line(&record)), "{stdout}");
}
