use rullst_ai::{
    AiClient, DurableAuditError, DurableToolAuditTrail, ToolAuditEvent, ToolAuditOutcome,
    ToolAuditSink, ToolExecutionError, ToolRisk,
    ai::{
        providers::openai::OpenAiProvider,
        rag::{
            DurableRagAuditTrail, InMemoryRagRetriever, RagAuditEvent, RagAuditOutcome,
            RagAuditSink, RagDocument, RagPipeline,
        },
    },
};
use rullst_core::security::TenantContext;
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

fn temporary_audit(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-ai-audit-{label}-{}-{nonce}.log",
        std::process::id()
    ))
}

fn rag_event(index: usize) -> RagAuditEvent {
    RagAuditEvent {
        tenant_id: "acme".to_string(),
        query_sha256: format!("{index:064x}"),
        retrieved_documents: 2,
        included_documents: 1,
        context_chars: 42,
        outcome: RagAuditOutcome::Succeeded,
    }
}

fn tool_event(index: usize) -> ToolAuditEvent {
    ToolAuditEvent {
        principal: format!("operator-{index}"),
        tool: "publish_report".to_string(),
        risk: Some(ToolRisk::Mutating),
        approved_by: None,
        approval_reason: None,
        outcome: ToolAuditOutcome::Succeeded,
    }
}

#[test]
fn rag_and_tool_audits_survive_restart_with_separate_versioned_streams() {
    let rag_path = temporary_audit("rag-restart");
    let tool_path = temporary_audit("tool-restart");
    {
        let rag = DurableRagAuditTrail::try_open(&rag_path).expect("create RAG audit");
        rag.record(rag_event(1)).expect("record first RAG event");
        rag.record(rag_event(2)).expect("record second RAG event");

        let tools = DurableToolAuditTrail::try_open(&tool_path).expect("create tool audit");
        tools
            .record(tool_event(1))
            .expect("record first tool event");
    }

    let rag = DurableRagAuditTrail::try_open(&rag_path).expect("reopen RAG audit");
    let rag_entries = rag.entries().expect("read RAG entries");
    assert_eq!(rag_entries.len(), 2);
    assert_eq!(rag_entries[0].sequence, 1);
    assert_eq!(rag_entries[1].sequence, 2);
    assert_eq!(rag_entries[1].event, rag_event(2));
    assert_eq!(rag.snapshot().expect("RAG snapshot").records(), 2);

    let tools = DurableToolAuditTrail::try_open(&tool_path).expect("reopen tool audit");
    let tool_entries = tools.entries().expect("read tool entries");
    assert_eq!(tool_entries.len(), 1);
    assert_eq!(tool_entries[0].sequence, 1);
    assert_eq!(tool_entries[0].event, tool_event(1));

    drop(rag);
    drop(tools);
    assert!(matches!(
        DurableToolAuditTrail::try_open(&rag_path),
        Err(DurableAuditError::CorruptRecord { record: 0, .. })
    ));
    assert!(matches!(
        DurableRagAuditTrail::try_open(&tool_path),
        Err(DurableAuditError::CorruptRecord { record: 0, .. })
    ));
    std::fs::remove_file(rag_path).expect("remove RAG audit");
    std::fs::remove_file(tool_path).expect("remove tool audit");
}

#[test]
fn same_length_tampering_is_detected_on_restart() {
    let path = temporary_audit("tamper");
    {
        let audit = DurableRagAuditTrail::try_open(&path).expect("create audit");
        audit.record(rag_event(7)).expect("record event");
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("open audit for tampering");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read audit");
    let position = bytes
        .windows(b"acme".len())
        .position(|window| window == b"acme")
        .expect("fixture tenant should exist");
    bytes[position] = b'A';
    file.seek(SeekFrom::Start(0)).expect("seek audit");
    file.write_all(&bytes).expect("rewrite audit");
    file.sync_data().expect("sync tampering");
    drop(file);

    assert!(matches!(
        DurableRagAuditTrail::try_open(&path),
        Err(DurableAuditError::CorruptRecord {
            record: 1,
            reason: "digest mismatch"
        })
    ));
    std::fs::remove_file(path).expect("remove audit");
}

#[test]
fn external_growth_and_competing_writers_fail_closed() {
    let path = temporary_audit("external");
    let first = DurableToolAuditTrail::try_open(&path).expect("open first writer");
    let second = DurableToolAuditTrail::try_open(&path).expect("open second writer");
    first.record(tool_event(1)).expect("first writer appends");
    assert!(matches!(
        second.record(tool_event(2)),
        Err(ToolExecutionError::AuditUnavailable(message))
            if message.contains("changed outside")
    ));
    assert_eq!(
        second.snapshot(),
        Err(DurableAuditError::ExternalModification)
    );
    drop(first);
    drop(second);
    std::fs::remove_file(path).expect("remove audit");
}

#[test]
fn byte_quota_and_invalid_events_preserve_the_previous_file() {
    let path = temporary_audit("quota");
    let audit =
        DurableToolAuditTrail::try_open_with_max_bytes(&path, 512).expect("create bounded audit");
    let mut exhausted = false;
    for index in 0..32 {
        let before = audit.snapshot().expect("snapshot before append");
        match audit.record(tool_event(index)) {
            Ok(()) => {}
            Err(ToolExecutionError::AuditUnavailable(message)) => {
                assert!(message.contains("capacity is exhausted"));
                assert_eq!(audit.snapshot().expect("snapshot after rejection"), before);
                exhausted = true;
                break;
            }
            Err(error) => panic!("unexpected append error: {error}"),
        }
    }
    assert!(exhausted, "small quota should reject a bounded append");
    drop(audit);

    let reopened =
        DurableToolAuditTrail::try_open_with_max_bytes(&path, 512).expect("reopen bounded audit");
    assert!(
        !reopened
            .entries()
            .expect("read retained entries")
            .is_empty()
    );
    drop(reopened);
    std::fs::remove_file(path).expect("remove bounded audit");

    let invalid_path = temporary_audit("invalid");
    let rag = DurableRagAuditTrail::try_open(&invalid_path).expect("create RAG audit");
    let mut invalid = rag_event(1);
    invalid.query_sha256 = "not-a-digest".to_string();
    assert!(rag.record(invalid).is_err());
    assert_eq!(rag.snapshot().expect("empty snapshot").records(), 0);
    drop(rag);
    std::fs::remove_file(invalid_path).expect("remove invalid audit");
}

#[test]
fn concurrent_in_process_appends_are_sequence_complete() {
    let path = temporary_audit("concurrent");
    let audit = Arc::new(DurableRagAuditTrail::try_open(&path).expect("create audit"));
    let handles = (1..=32)
        .map(|index| {
            let audit = Arc::clone(&audit);
            std::thread::spawn(move || audit.record(rag_event(index)))
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("append thread").expect("append event");
    }
    let entries = audit.entries().expect("read concurrent entries");
    assert_eq!(entries.len(), 32);
    assert_eq!(entries.first().expect("first entry").sequence, 1);
    assert_eq!(entries.last().expect("last entry").sequence, 32);
    drop(audit);
    std::fs::remove_file(path).expect("remove audit");
}

#[tokio::test]
async fn real_rag_pipeline_records_a_durable_success_that_survives_restart() {
    let path = temporary_audit("pipeline");
    let tenant = TenantContext::try_new("tenant:acme").expect("tenant");
    let client = AiClient::new(OpenAiProvider::new("mock_durable_rag"));
    let content = "Preview a framework upgrade with cargo rullst upgrade --dry-run.";
    let vector = client.embed(content).await.expect("offline embedding");
    let retriever = InMemoryRagRetriever::try_new(8, vector.len()).expect("retriever");
    retriever
        .upsert(
            &tenant,
            RagDocument::try_new(&tenant, "upgrade-guide", content, 0.0).expect("document"),
            vector,
        )
        .expect("index document");
    let audit = Arc::new(DurableRagAuditTrail::try_open(&path).expect("durable audit"));
    let pipeline = RagPipeline::new(client, retriever, Arc::clone(&audit));

    let answer = pipeline
        .answer(&tenant, "How do I preview an upgrade?")
        .await
        .expect("offline grounded answer");
    assert_eq!(answer.sources()[0].document_id(), "upgrade-guide");
    drop(pipeline);
    drop(audit);

    let reopened = DurableRagAuditTrail::try_open(&path).expect("reopen durable audit");
    let entries = reopened.entries().expect("read durable audit");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].event.tenant_id, "tenant:acme");
    assert_eq!(entries[0].event.outcome, RagAuditOutcome::Succeeded);
    drop(reopened);
    std::fs::remove_file(path).expect("remove audit");
}

#[cfg(unix)]
#[test]
fn symlink_targets_are_rejected() {
    use std::os::unix::fs::symlink;

    let target = temporary_audit("target");
    let link = temporary_audit("link");
    std::fs::File::create(&target).expect("create target");
    symlink(&target, &link).expect("create symlink");
    assert!(matches!(
        DurableRagAuditTrail::try_open(&link),
        Err(DurableAuditError::UnsafeFileType)
    ));
    std::fs::remove_file(link).expect("remove link");
    std::fs::remove_file(target).expect("remove target");
}
