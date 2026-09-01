//! Server-side upload admission and quarantine contracts.
//!
//! This module validates a bounded in-memory object before storage. It does not
//! implement multipart streaming, sandboxed document parsing, or a production
//! malware engine; applications must provide those boundaries separately.

/// Upload validation and scan failures.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum UploadError {
    /// The configured policy is empty, inconsistent, or unreasonably broad.
    #[error("invalid upload policy: {0}")]
    InvalidPolicy(String),
    /// The authenticated tenant identifier is absent or malformed.
    #[error("invalid upload tenant")]
    InvalidTenant,
    /// The client filename is unsafe or disagrees with detected content.
    #[error("invalid upload filename")]
    InvalidFileName,
    /// The object is empty or exceeds the configured byte limit.
    #[error("upload size is outside policy")]
    InvalidSize,
    /// Declared and detected media types disagree or the type is not allowed.
    #[error("upload media type is denied")]
    MediaTypeDenied,
    /// Active or structurally dangerous content was recognized.
    #[error("active upload content is denied")]
    ActiveContentDenied,
    /// A scanner reported malicious content.
    #[error("upload scanner rejected the object")]
    MalwareDetected,
    /// No trustworthy clean verdict was available.
    #[error("upload scanner did not return a clean verdict")]
    ScanUnavailable,
}

/// Bounded content families recognized by the built-in signature gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum UploadKind {
    /// Portable Network Graphics image recognized by its fixed signature.
    Png,
    /// JPEG image recognized by its start-of-image signature.
    Jpeg,
    /// Portable Document Format object recognized by its header.
    Pdf,
    /// ISO base media object with an `ftyp` box in the expected position.
    Mp4,
    /// WebM/Matroska object recognized by its EBML header.
    WebM,
    /// UTF-8 plain text without recognized active-content prefixes.
    PlainText,
}

impl UploadKind {
    /// Returns the canonical media type required at admission.
    pub fn media_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Pdf => "application/pdf",
            Self::Mp4 => "video/mp4",
            Self::WebM => "video/webm",
            Self::PlainText => "text/plain",
        }
    }

    /// Returns the canonical storage extension for this kind.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Pdf => "pdf",
            Self::Mp4 => "mp4",
            Self::WebM => "webm",
            Self::PlainText => "txt",
        }
    }
}

/// Immutable upload admission policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UploadPolicy {
    max_bytes: usize,
    allowed: Vec<UploadKind>,
}

impl UploadPolicy {
    /// Constructs a bounded allowlist. The hard ceiling prevents accidentally
    /// buffering objects larger than 100 MiB through this in-memory API.
    pub fn try_new(
        max_bytes: usize,
        allowed: impl IntoIterator<Item = UploadKind>,
    ) -> Result<Self, UploadError> {
        let mut allowed = allowed.into_iter().collect::<Vec<_>>();
        allowed.sort_unstable();
        allowed.dedup();
        if !(1..=100 * 1024 * 1024).contains(&max_bytes) || allowed.is_empty() {
            return Err(UploadError::InvalidPolicy(
                "size and at least one allowed type are required".to_string(),
            ));
        }
        Ok(Self { max_bytes, allowed })
    }

    /// Returns the maximum admitted byte length.
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Returns the deduplicated content-kind allowlist.
    pub fn allowed_kinds(&self) -> &[UploadKind] {
        &self.allowed
    }

    /// Validates identity, name, size, declared MIME and signature before an
    /// application writes the bytes under the returned quarantine key.
    pub fn admit(
        &self,
        tenant_id: &str,
        file_name: &str,
        declared_media_type: &str,
        bytes: &[u8],
    ) -> Result<QuarantinedUpload, UploadError> {
        if !valid_tenant_id(tenant_id) {
            return Err(UploadError::InvalidTenant);
        }
        validate_file_name(file_name)?;
        if bytes.is_empty() || bytes.len() > self.max_bytes {
            return Err(UploadError::InvalidSize);
        }
        let kind = detect_kind(bytes)?;
        if !self.allowed.contains(&kind)
            || declared_media_type.trim().to_ascii_lowercase() != kind.media_type()
            || !extension_matches(file_name, kind)
        {
            return Err(UploadError::MediaTypeDenied);
        }
        let object_id = uuid::Uuid::new_v4();
        let quarantine_key = format!("quarantine/{tenant_id}/{object_id}.{}", kind.extension());
        let digest = ring::digest::digest(&ring::digest::SHA256, bytes);
        Ok(QuarantinedUpload {
            quarantine_key,
            display_name: file_name.to_string(),
            kind,
            byte_len: bytes.len(),
            sha256_hex: encode_hex(digest.as_ref()),
        })
    }
}

/// Metadata for an admitted object that must remain quarantined until scanned.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QuarantinedUpload {
    quarantine_key: String,
    display_name: String,
    kind: UploadKind,
    byte_len: usize,
    sha256_hex: String,
}

impl QuarantinedUpload {
    /// Returns the tenant-prefixed temporary object key.
    pub fn quarantine_key(&self) -> &str {
        &self.quarantine_key
    }

    /// Returns the validated, display-only client filename.
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the content kind detected from the bytes.
    pub fn kind(&self) -> UploadKind {
        self.kind
    }

    /// Returns the admitted byte length.
    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// Returns the lowercase SHA-256 digest of the admitted bytes.
    pub fn sha256_hex(&self) -> &str {
        &self.sha256_hex
    }

    /// A release is possible only from an explicit clean scanner verdict.
    pub fn release(self, verdict: ScanVerdict) -> Result<ReleasedUpload, UploadError> {
        match verdict {
            ScanVerdict::Clean {
                engine,
                evidence_id,
            } if valid_evidence(&engine) && valid_evidence(&evidence_id) => Ok(ReleasedUpload {
                object_key: self.quarantine_key.replacen("quarantine/", "accepted/", 1),
                display_name: self.display_name,
                kind: self.kind,
                byte_len: self.byte_len,
                sha256_hex: self.sha256_hex,
                scanner_engine: engine,
                scan_evidence_id: evidence_id,
            }),
            ScanVerdict::Infected { .. } => Err(UploadError::MalwareDetected),
            ScanVerdict::Unavailable { .. } | ScanVerdict::Clean { .. } => {
                Err(UploadError::ScanUnavailable)
            }
        }
    }
}

/// Scanner outcome. `Unavailable` is always fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScanVerdict {
    /// The scanner completed and did not recognize malicious content.
    Clean {
        /// Stable scanner or adapter identifier.
        engine: String,
        /// Non-empty audit reference supplied by the scanner adapter.
        evidence_id: String,
    },
    /// The scanner recognized malicious content.
    Infected {
        /// Stable scanner or adapter identifier.
        engine: String,
        /// Scanner-specific malware signature or rule identifier.
        signature: String,
    },
    /// The scanner could not produce a trustworthy verdict.
    Unavailable {
        /// Stable scanner or adapter identifier.
        engine: String,
        /// Diagnostic reason suitable for structured logs.
        reason: String,
    },
}

/// Static-dispatch scanner adapter contract.
pub trait UploadScanner {
    /// Scans bytes previously admitted as `upload` and returns an explicit verdict.
    fn scan(&self, upload: &QuarantinedUpload, bytes: &[u8]) -> Result<ScanVerdict, UploadError>;
}

/// Deterministic offline scanner for tests and local development only.
#[derive(Debug, Clone, Copy, Default)]
pub struct OfflineMockScanner;

impl UploadScanner for OfflineMockScanner {
    fn scan(&self, upload: &QuarantinedUpload, bytes: &[u8]) -> Result<ScanVerdict, UploadError> {
        if bytes.len() != upload.byte_len {
            return Err(UploadError::InvalidSize);
        }
        let digest = ring::digest::digest(&ring::digest::SHA256, bytes);
        if encode_hex(digest.as_ref()) != upload.sha256_hex {
            return Err(UploadError::ScanUnavailable);
        }
        if bytes.windows(5).any(|window| window == b"EICAR") {
            return Ok(ScanVerdict::Infected {
                engine: "offline-mock-v1".to_string(),
                signature: "deterministic-eicar-marker".to_string(),
            });
        }
        Ok(ScanVerdict::Clean {
            engine: "offline-mock-v1".to_string(),
            evidence_id: upload.sha256_hex.clone(),
        })
    }
}

/// Metadata safe to persist after a clean scan.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReleasedUpload {
    /// Tenant-prefixed object key outside the quarantine namespace.
    pub object_key: String,
    /// Validated, display-only original filename.
    pub display_name: String,
    /// Content kind detected during admission.
    pub kind: UploadKind,
    /// Admitted byte length.
    pub byte_len: usize,
    /// Lowercase SHA-256 digest of the admitted bytes.
    pub sha256_hex: String,
    /// Scanner or adapter identifier that produced the clean verdict.
    pub scanner_engine: String,
    /// Scanner evidence reference retained for audit.
    pub scan_evidence_id: String,
}

fn valid_tenant_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn validate_file_name(value: &str) -> Result<(), UploadError> {
    if value.is_empty()
        || value.len() > 255
        || value == "."
        || value == ".."
        || value.contains(['/', '\\', '\0'])
        || value.chars().any(char::is_control)
    {
        return Err(UploadError::InvalidFileName);
    }
    Ok(())
}

fn extension_matches(file_name: &str, kind: UploadKind) -> bool {
    let extension = file_name.rsplit_once('.').map(|(_, value)| value);
    match kind {
        UploadKind::Jpeg => extension.is_some_and(|value| {
            value.eq_ignore_ascii_case("jpg") || value.eq_ignore_ascii_case("jpeg")
        }),
        _ => extension.is_some_and(|value| value.eq_ignore_ascii_case(kind.extension())),
    }
}

fn detect_kind(bytes: &[u8]) -> Result<UploadKind, UploadError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(UploadKind::Png);
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Ok(UploadKind::Jpeg);
    }
    if bytes.starts_with(b"%PDF-") {
        return Ok(UploadKind::Pdf);
    }
    if bytes.len() >= 12 && bytes.get(4..8) == Some(b"ftyp") {
        return Ok(UploadKind::Mp4);
    }
    if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        return Ok(UploadKind::WebM);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| UploadError::MediaTypeDenied)?;
    if text.chars().any(|character| {
        character == '\0' || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    }) {
        return Err(UploadError::MediaTypeDenied);
    }
    let normalized = text.trim_start().to_ascii_lowercase();
    if ["<svg", "<html", "<!doctype", "<script"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        || normalized.contains("javascript:")
    {
        return Err(UploadError::ActiveContentDenied);
    }
    Ok(UploadKind::PlainText)
}

fn valid_evidence(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn text_policy() -> UploadPolicy {
        UploadPolicy::try_new(1_024, [UploadKind::PlainText]).expect("test upload policy")
    }

    #[test]
    // TM-ACADEMY-09
    fn upload_policy_rejects_spoofing_active_content_and_unsafe_names() {
        let policy = text_policy();
        assert!(matches!(
            policy.admit(
                "school-1",
                "note.txt",
                "text/plain",
                b"<svg onload=alert(1)>"
            ),
            Err(UploadError::ActiveContentDenied)
        ));
        assert!(matches!(
            policy.admit("school-1", "../note.txt", "text/plain", b"safe"),
            Err(UploadError::InvalidFileName)
        ));
        assert!(matches!(
            policy.admit("school-1", "note.pdf", "application/pdf", b"plain text"),
            Err(UploadError::MediaTypeDenied)
        ));
        assert!(matches!(
            policy.admit("../school", "note.txt", "text/plain", b"safe"),
            Err(UploadError::InvalidTenant)
        ));
    }

    #[test]
    fn quarantine_requires_a_clean_digest_bound_scan() {
        let policy = text_policy();
        let bytes = b"bounded learner submission";
        let admitted = policy
            .admit("school-1", "answer.txt", "text/plain", bytes)
            .expect("admitted text");
        assert!(
            admitted
                .quarantine_key()
                .starts_with("quarantine/school-1/")
        );
        let verdict = OfflineMockScanner
            .scan(&admitted, bytes)
            .expect("offline clean scan");
        let released = admitted.release(verdict).expect("clean release");
        assert!(released.object_key.starts_with("accepted/school-1/"));
        assert_eq!(released.byte_len, bytes.len());

        let infected_bytes = b"EICAR deterministic fixture";
        let infected = policy
            .admit("school-1", "fixture.txt", "text/plain", infected_bytes)
            .expect("admitted infected fixture");
        let verdict = OfflineMockScanner
            .scan(&infected, infected_bytes)
            .expect("offline infected scan");
        assert!(matches!(
            infected.release(verdict),
            Err(UploadError::MalwareDetected)
        ));
    }

    #[test]
    fn unavailable_or_tampered_scan_fails_closed() {
        let admitted = text_policy()
            .admit("school-1", "answer.txt", "text/plain", b"original")
            .expect("admitted text");
        assert!(matches!(
            OfflineMockScanner.scan(&admitted, b"changed"),
            Err(UploadError::InvalidSize | UploadError::ScanUnavailable)
        ));
        assert!(matches!(
            admitted.release(ScanVerdict::Unavailable {
                engine: "scanner".to_string(),
                reason: "offline".to_string(),
            }),
            Err(UploadError::ScanUnavailable)
        ));
    }
}

#[cfg(test)]
#[path = "uploads_contract_tests.rs"]
mod contract_tests;
