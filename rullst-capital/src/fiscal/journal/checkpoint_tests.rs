use super::*;

#[test]
fn retained_checkpoint_is_verified_as_an_authenticated_prefix() {
    let file = TempJournal::new("checkpoint-prefix");
    let journal = FiscalCommandJournal::try_open(file.path(), key(17)).expect("open journal");
    let empty = journal.checkpoint().expect("empty checkpoint");
    journal
        .prepare_at("invoice:a", NfseEnvironment::Homologation, request(), 1)
        .expect("first preparation");
    let retained = journal.checkpoint().expect("retained checkpoint");
    // The process crashes after this append but before persisting a new checkpoint.
    journal
        .prepare_at("invoice:b", NfseEnvironment::Homologation, request(), 2)
        .expect("second preparation");
    let tip = journal.checkpoint().expect("tip checkpoint");
    drop(journal);

    let reopened = FiscalCommandJournal::try_open(file.path(), key(17)).expect("reopen");
    assert_eq!(
        reopened.verify_checkpoint(&retained),
        Err(FiscalJournalError::CheckpointMismatch)
    );
    assert_eq!(reopened.verify_checkpoint_prefix(&retained), Ok(1));
    assert_eq!(reopened.verify_checkpoint_prefix(&empty), Ok(2));
    assert_eq!(reopened.verify_checkpoint_prefix(&tip), Ok(0));

    let mut forged = retained.clone();
    forged.commitment = "0".repeat(64);
    assert_eq!(
        reopened.verify_checkpoint_prefix(&forged),
        Err(FiscalJournalError::CheckpointMismatch)
    );
    let mut shifted = retained.clone();
    shifted.end_offset += 1;
    assert_eq!(
        reopened.verify_checkpoint_prefix(&shifted),
        Err(FiscalJournalError::CheckpointMismatch)
    );
    drop(reopened);

    // A valid-prefix truncation is still detected against the newer checkpoint.
    let bytes = std::fs::read(file.path()).expect("journal bytes");
    let keep = usize::try_from(retained.end_offset()).expect("offset");
    std::fs::write(file.path(), &bytes[..keep]).expect("truncate journal");
    let truncated = FiscalCommandJournal::try_open(file.path(), key(17)).expect("truncated");
    assert_eq!(
        truncated.verify_checkpoint_prefix(&tip),
        Err(FiscalJournalError::CheckpointMismatch)
    );
    assert_eq!(truncated.verify_checkpoint_prefix(&retained), Ok(0));
}
