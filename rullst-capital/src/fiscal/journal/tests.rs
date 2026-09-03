use super::*;
use crate::fiscal::{NFSE_NAMESPACE, NfseIssueRequest, NfseIssueResponse};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::{Compression, GzBuilder};
use rcgen::{CertificateParams, KeyPair, PKCS_RSA_SHA256};
use serde_json::json;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use xml_sec::{
    c14n::{C14nAlgorithm, C14nMode},
    xmldsig::{
        DigestAlgorithm, ReferenceBuilder, RsaSigningKey, SignContext, SignatureAlgorithm,
        SignatureBuilder, Transform, X509CertificateKeyInfoWriter,
    },
};

const DPS_ID: &str = "DPS355030821122233300018100001000000000000101";
const SECOND_DPS_ID: &str = "DPS355030821122233300018100001000000000000102";
const ACCESS_KEY: &str = "35503082112223330001810000100000000000010112345678";

struct TempJournal(PathBuf);

impl TempJournal {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rullst-capital-nfse-{label}-{}-{sequence}.journal",
            std::process::id()
        ));
        let _cleanup = std::fs::remove_file(&path);
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempJournal {
    fn drop(&mut self) {
        let _cleanup = std::fs::remove_file(&self.0);
    }
}

fn key(byte: u8) -> FiscalJournalKey {
    FiscalJournalKey::try_new("fiscal-2026-01", [byte; 32]).expect("journal key")
}

fn request() -> &'static NfseIssueRequest {
    static REQUEST: OnceLock<NfseIssueRequest> = OnceLock::new();
    REQUEST.get_or_init(|| {
        NfseIssueRequest::try_from_signed_dps(signed_dps(DPS_ID)).expect("signed request")
    })
}

fn second_request() -> &'static NfseIssueRequest {
    static REQUEST: OnceLock<NfseIssueRequest> = OnceLock::new();
    REQUEST.get_or_init(|| {
        NfseIssueRequest::try_from_signed_dps(signed_dps(SECOND_DPS_ID))
            .expect("second signed request")
    })
}

fn authorization() -> NfseIssueResponse {
    static XML: OnceLock<String> = OnceLock::new();
    let xml = XML.get_or_init(|| {
        let id = format!("NFS{ACCESS_KEY}");
        sign_fixture(
            &format!(
                "<NFSe xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infNFSe Id=\"{id}\"/></NFSe>"
            ),
            &id,
        )
    });
    let body = serde_json::to_vec(&json!({
        "tipoAmbiente": 2,
        "versaoAplicativo": "SefinNacional_1.0",
        "dataHoraProcessamento": "2026-08-30T21:47:12-03:00",
        "idDps": DPS_ID,
        "chaveAcesso": ACCESS_KEY,
        "nfseXmlGZipB64": encoded_xml(xml),
        "alertas": [{"codigo": "A001", "descricao": "Fixture warning"}]
    }))
    .expect("authorization JSON");
    request()
        .parse_response(201, NfseEnvironment::Homologation, &body)
        .expect("authorization")
}

fn rejection() -> NfseIssueResponse {
    let body = serde_json::to_vec(&json!({
        "tipoAmbiente": 2,
        "versaoAplicativo": "SefinNacional_1.0",
        "dataHoraProcessamento": "2026-08-30T21:47:12-03:00",
        "idDPS": DPS_ID,
        "erros": [{"Codigo": "E0712", "Descricao": "Rejected fixture"}]
    }))
    .expect("rejection JSON");
    request()
        .parse_response(400, NfseEnvironment::Homologation, &body)
        .expect("rejection")
}

fn signed_dps(id: &str) -> &'static str {
    if id == DPS_ID {
        static XML: OnceLock<String> = OnceLock::new();
        XML.get_or_init(|| signed_dps_owned(DPS_ID))
    } else {
        static XML: OnceLock<String> = OnceLock::new();
        XML.get_or_init(|| signed_dps_owned(SECOND_DPS_ID))
    }
}

fn signed_dps_owned(id: &str) -> String {
    sign_fixture(
        &format!(
            "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{id}\"><tpAmb>2</tpAmb></infDPS></DPS>"
        ),
        id,
    )
}

fn sign_fixture(xml: &str, id: &str) -> String {
    let key_pair = KeyPair::generate_for(&PKCS_RSA_SHA256).expect("RSA keypair");
    let certificate = CertificateParams::new(vec!["nfse.test".to_string()])
        .expect("certificate params")
        .self_signed(&key_pair)
        .expect("test certificate");
    let signing_key =
        RsaSigningKey::from_pkcs8_der(key_pair.serialized_der()).expect("RSA signing key");
    let key_info = X509CertificateKeyInfoWriter::from_der_chain([certificate.der().as_ref()])
        .expect("X.509 key info");
    let c14n = C14nAlgorithm::new(C14nMode::Inclusive1_0, false);
    let template = SignatureBuilder::new(c14n.clone(), SignatureAlgorithm::RsaSha256)
        .add_reference(
            ReferenceBuilder::new(DigestAlgorithm::Sha256)
                .uri(format!("#{id}"))
                .transform(Transform::Enveloped)
                .transform(Transform::C14n(c14n)),
        )
        .key_info(true)
        .build_template()
        .expect("signature template");
    let with_template = xml_sec::xmldsig::mutation::append_signature_to_root(xml, &template)
        .expect("append signature");
    SignContext::new(&signing_key)
        .key_info_writer(&key_info)
        .sign_template(&with_template)
        .expect("sign fixture")
}

fn encoded_xml(xml: &str) -> String {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).expect("write GZip");
    STANDARD.encode(encoder.finish().expect("finish GZip"))
}

#[test]
fn prepared_authorized_lifecycle_is_idempotent_minimized_and_restart_safe() {
    let file = TempJournal::new("lifecycle");
    let journal = FiscalCommandJournal::try_open(file.path(), key(7)).expect("open journal");

    let prepared = journal
        .prepare_at(
            "invoice:42",
            NfseEnvironment::Homologation,
            request(),
            1_000,
        )
        .expect("prepare");
    assert_eq!(prepared.disposition(), FiscalJournalDisposition::Recorded);
    assert_eq!(prepared.status(), FiscalCommandStatus::Prepared);
    assert_eq!(prepared.sequence(), 1);
    assert_eq!(journal.pending().expect("pending").len(), 1);

    let replay = journal
        .prepare_at(
            "invoice:42",
            NfseEnvironment::Homologation,
            request(),
            2_000,
        )
        .expect("prepare replay");
    assert_eq!(replay.disposition(), FiscalJournalDisposition::Replay);
    assert_eq!(replay.sequence(), 1);

    let response = authorization();
    let terminal = journal
        .record_response_at("invoice:42", request(), &response, 3_000)
        .expect("record authorization");
    assert_eq!(terminal.status(), FiscalCommandStatus::Authorized);
    assert_eq!(terminal.sequence(), 2);
    let replay = journal
        .record_response_at("invoice:42", request(), &response, 4_000)
        .expect("authorization replay");
    assert_eq!(replay.disposition(), FiscalJournalDisposition::Replay);
    assert_eq!(journal.pending().expect("pending").len(), 0);
    assert_eq!(journal.snapshot().expect("snapshot").records(), 2);

    let checkpoint = journal.checkpoint().expect("checkpoint");
    let encoded = serde_json::to_vec(&checkpoint).expect("serialize checkpoint");
    let checkpoint: FiscalJournalCheckpoint =
        serde_json::from_slice(&encoded).expect("deserialize checkpoint");
    drop(journal);

    let bytes = std::fs::read(file.path()).expect("journal bytes");
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains(DPS_ID));
    assert!(!text.contains(ACCESS_KEY));
    assert!(!text.contains("nfseXmlGZipB64"));
    assert!(!text.contains("Fixture warning"));

    let reopened = FiscalCommandJournal::try_open(file.path(), key(7)).expect("reopen");
    reopened
        .verify_checkpoint(&checkpoint)
        .expect("checkpoint survives restart");
    assert_eq!(
        reopened.status("invoice:42").expect("status"),
        Some(FiscalCommandStatus::Authorized)
    );
}

#[test]
fn pending_recovery_and_conflicting_transitions_fail_closed() {
    let file = TempJournal::new("recovery");
    let journal = FiscalCommandJournal::try_open(file.path(), key(8)).expect("open journal");
    journal
        .prepare_at("invoice:one", NfseEnvironment::Homologation, request(), 10)
        .expect("first preparation");
    journal
        .prepare_at("invoice:two", NfseEnvironment::Homologation, request(), 20)
        .expect("second preparation");

    assert_eq!(
        journal.prepare_at(
            "invoice:one",
            NfseEnvironment::Homologation,
            second_request(),
            30
        ),
        Err(FiscalJournalError::CommandConflict)
    );
    assert_eq!(
        journal.prepare_at("mock", NfseEnvironment::Mock, request(), 30),
        Err(FiscalJournalError::InvalidEnvironment)
    );
    assert_eq!(
        journal.prepare_at(
            "wrong-environment",
            NfseEnvironment::Production,
            request(),
            30
        ),
        Err(FiscalJournalError::ResponseMismatch)
    );
    assert_eq!(
        journal.record_response_at("missing", request(), &rejection(), 30),
        Err(FiscalJournalError::MissingCommand)
    );
    assert_eq!(
        journal.record_response_at("invoice:one", request(), &rejection(), 9),
        Err(FiscalJournalError::ResponseMismatch)
    );

    journal
        .record_response_at("invoice:one", request(), &rejection(), 30)
        .expect("record rejection");
    assert_eq!(
        journal.record_response_at("invoice:one", request(), &authorization(), 40),
        Err(FiscalJournalError::CommandConflict)
    );
    let pending = journal.pending().expect("pending recovery");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].command_id(), "invoice:two");
    assert_eq!(pending[0].sequence(), 2);
    assert_eq!(pending[0].prepared_at_unix_ms(), 20);
    assert_eq!(pending[0].environment(), NfseEnvironment::Homologation);
    assert_eq!(pending[0].request_digest().len(), 64);
    assert!(!format!("{pending:?}").contains("invoice:two"));
}

#[test]
fn authentication_tampering_capacity_and_competing_writers_are_explicit() {
    let file = TempJournal::new("tamper");
    let first = FiscalCommandJournal::try_open(file.path(), key(9)).expect("first writer");
    let second = FiscalCommandJournal::try_open(file.path(), key(9)).expect("second writer");
    let empty_checkpoint = second.checkpoint().expect("empty checkpoint");
    first
        .prepare_at("invoice:7", NfseEnvironment::Homologation, request(), 1)
        .expect("prepare");
    assert_eq!(
        second.snapshot(),
        Err(FiscalJournalError::ExternalModification)
    );
    assert_eq!(
        first.verify_checkpoint(&empty_checkpoint),
        Err(FiscalJournalError::CheckpointMismatch)
    );
    let current = first.checkpoint().expect("current checkpoint");
    first.verify_checkpoint(&current).expect("current tip");
    drop(first);
    drop(second);

    assert!(matches!(
        FiscalCommandJournal::try_open(file.path(), key(10)),
        Err(FiscalJournalError::KeyMismatch)
    ));
    let mut bytes = std::fs::read(file.path()).expect("journal bytes");
    let marker = b"prepared";
    let position = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("prepared marker");
    bytes[position] = b'q';
    std::fs::write(file.path(), bytes).expect("tamper journal");
    assert!(matches!(
        FiscalCommandJournal::try_open(file.path(), key(9)),
        Err(FiscalJournalError::CorruptRecord { .. })
    ));

    let bounded_file = TempJournal::new("capacity");
    let bounded = FiscalCommandJournal::try_open_with_max_bytes(
        bounded_file.path(),
        key(11),
        MIN_FISCAL_JOURNAL_BYTES,
    )
    .expect("bounded journal");
    bounded
        .prepare_at(
            "invoice:capacity",
            NfseEnvironment::Homologation,
            request(),
            1,
        )
        .expect("first bounded record");
    assert_eq!(
        bounded.prepare_at(
            "invoice:overflow",
            NfseEnvironment::Homologation,
            request(),
            2,
        ),
        Err(FiscalJournalError::CapacityExceeded)
    );
    assert!(FiscalJournalKey::try_new("bad:key", [1_u8; 32]).is_err());
    assert!(FiscalJournalKey::try_new("key", [1_u8; 31]).is_err());
    assert_eq!(
        bounded.status("contains whitespace"),
        Err(FiscalJournalError::InvalidCommandId)
    );
}

#[cfg(unix)]
#[test]
fn symbolic_link_targets_are_rejected() {
    use std::os::unix::fs::symlink;

    let target = TempJournal::new("symlink-target");
    std::fs::write(target.path(), []).expect("create target");
    let link = TempJournal::new("symlink-link");
    symlink(target.path(), link.path()).expect("create link");
    assert!(matches!(
        FiscalCommandJournal::try_open(link.path(), key(12)),
        Err(FiscalJournalError::UnsafeFileType)
    ));
}

#[test]
fn public_debug_and_errors_do_not_disclose_sensitive_inputs() {
    let secret_path = "/tmp/customer-52998224725-secret.journal";
    let error = FiscalCommandJournal::try_open(
        PathBuf::from(secret_path).join("missing-parent/file"),
        key(13),
    )
    .err()
    .expect("open error");
    let diagnostic = format!("{error:?} {error}");
    assert!(!diagnostic.contains("52998224725"));
    assert!(!format!("{:?}", key(13)).contains("13, 13"));
}
