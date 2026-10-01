#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let suffix = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rullst-nfse-schema-{}-{suffix}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(digest(&SHA256, bytes).as_ref())
}

fn validator() -> NfseDpsSchemaValidator {
    let xsd = concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema""#,
        r#" targetNamespace="http://www.sped.fazenda.gov.br/nfse" elementFormDefault="qualified">"#,
        r#"<xs:element name="DPS"><xs:complexType><xs:simpleContent>"#,
        r#"<xs:extension base="xs:string"><xs:attribute name="versao" type="xs:string" use="required"/>"#,
        r#"</xs:extension></xs:simpleContent></xs:complexType></xs:element>"#,
        r#"<xs:element name="Other" type="xs:string"/>"#,
        "</xs:schema>"
    );
    let mut set = SchemaSet::new();
    set.add_document(Some("pinned://DPS.xsd"), xsd).unwrap();
    NfseDpsSchemaValidator {
        schema: set.compile().unwrap(),
        profile: "synthetic-test",
    }
}

#[test]
fn manifests_are_exact_and_unknown_values_fail_before_io() {
    assert_eq!(
        expected_files(&NFSE_PRODUCTION_V1_01_20260209).unwrap(),
        &PRODUCTION_FILES
    );
    assert_eq!(
        expected_files(&NFSE_RESTRICTED_V1_01_20260727).unwrap(),
        &RESTRICTED_FILES
    );

    static UNKNOWN: NfseArtifactManifest = NfseArtifactManifest {
        profile: "unknown",
        schema_version: "1.01",
        artifact_date: "1970-01-01",
        schema_archive: "unknown.zip",
        schema_archive_sha256: "00",
        dps_root_schema: "DPS.xsd",
        nfse_root_schema: "NFSe.xsd",
        documentation_url: "https://example.invalid",
    };
    assert!(NfseDpsSchemaValidator::from_pinned_directory("/does/not/matter", &UNKNOWN).is_err());
}

#[test]
fn verified_document_loader_checks_missing_size_hash_utf8_and_bom() {
    let directory = TestDirectory::new();
    let missing = [("missing.xsd", "00")];
    assert!(load_verified_documents(&directory.0, &missing).is_err());

    let oversized = vec![b'x'; (MAX_XSD_FILE_BYTES + 1) as usize];
    std::fs::write(directory.0.join("oversized.xsd"), &oversized).unwrap();
    let oversized_files = [("oversized.xsd", "not-reached")];
    assert!(load_verified_documents(&directory.0, &oversized_files).is_err());

    std::fs::write(directory.0.join("mismatch.xsd"), b"schema").unwrap();
    let mismatched = [("mismatch.xsd", "00")];
    assert!(load_verified_documents(&directory.0, &mismatched).is_err());

    let invalid_utf8 = [0xff, 0xfe];
    std::fs::write(directory.0.join("invalid.xsd"), invalid_utf8).unwrap();
    let invalid_hash = sha256(&invalid_utf8);
    let invalid_files = [("invalid.xsd", invalid_hash.as_str())];
    assert!(load_verified_documents(&directory.0, &invalid_files).is_err());

    let bom_document = b"\xef\xbb\xbf<schema/>";
    std::fs::write(directory.0.join("valid.xsd"), bom_document).unwrap();
    let valid_hash = sha256(bom_document);
    let valid_files = [("valid.xsd", valid_hash.as_str())];
    let loaded = load_verified_documents(&directory.0, &valid_files).unwrap();
    assert_eq!(
        loaded.get("valid.xsd").map(String::as_str),
        Some("<schema/>")
    );
}

#[test]
fn resolver_is_catalogue_only_and_rejects_path_like_locations() {
    let mut resolver = PinnedResolver {
        documents: BTreeMap::from([("root.xsd".to_string(), "<schema/>".to_string())]),
    };
    let resolved = resolver.resolve("root.xsd", None).unwrap().unwrap();
    assert_eq!(resolved.uri, "pinned://root.xsd");
    assert_eq!(resolved.text, "<schema/>");
    assert!(resolver.resolve("unknown.xsd", None).unwrap().is_none());
    for unsafe_location in ["../root.xsd", "nested/root.xsd", r"nested\root.xsd"] {
        assert!(resolver.resolve(unsafe_location, None).unwrap().is_none());
    }
}

#[test]
fn compiled_validator_bounds_input_and_returns_structured_diagnostics() {
    let validator = validator();
    assert_eq!(validator.profile(), "synthetic-test");
    assert!(validator.validate(&dps("valid")).is_ok());

    for invalid in ["", "<!DOCTYPE DPS><DPS>invalid</DPS>"] {
        assert!(matches!(
            validator.validate(invalid),
            Err(FiscalError::InvalidInput {
                field: "dps.xml",
                ..
            })
        ));
    }
    let oversized = "x".repeat(MAX_DPS_XML_BYTES + 1);
    assert!(matches!(
        validator.validate(&oversized),
        Err(FiscalError::InvalidInput {
            field: "dps.xml",
            ..
        })
    ));

    let error = validator.validate(&dps("<nested/>")).unwrap_err();
    assert!(matches!(error, FiscalError::XmlValidation { .. }));
}

fn dps(content: &str) -> String {
    format!("<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\">{content}</DPS>")
}

#[test]
fn only_an_official_dps_root_is_accepted() {
    let validator = validator();
    for document in [
        "<DPS>unqualified</DPS>".to_string(),
        format!("<Other xmlns=\"{NFSE_NAMESPACE}\">declared but not a DPS</Other>"),
        format!("<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.00\">old</DPS>"),
        concat!(
            r#"<Foo xmlns:xs="http://www.w3.org/2001/XMLSchema""#,
            r#" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="xs:anyType">"#,
            "<anything/></Foo>"
        )
        .to_string(),
        format!(
            "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:type=\"xs:anyType\"><x/></DPS>"
        ),
        r#"<Signature xmlns="http://www.w3.org/2000/09/xmldsig#"/>"#.to_string(),
        "<DPS".to_string(),
    ] {
        assert!(
            matches!(
                validator.validate(&document),
                Err(FiscalError::InvalidInput {
                    field: "dps.xml",
                    ..
                })
            ),
            "{document}"
        );
    }
}

#[test]
fn compatibility_and_diagnostics_are_bounded() {
    let mut production = format!("<pattern {OFFICIAL_SERIES_PATTERN}/>");
    apply_pinned_regex_compatibility(&mut production, PRODUCTION_SIMPLE_TYPES_SHA256).unwrap();
    assert!(production.contains(XSD_COMPATIBLE_SERIES_PATTERN));
    assert!(!production.contains(OFFICIAL_SERIES_PATTERN));

    let mut unrelated = format!("<pattern {OFFICIAL_SERIES_PATTERN}/>");
    apply_pinned_regex_compatibility(&mut unrelated, "different-hash").unwrap();
    assert!(unrelated.contains(OFFICIAL_SERIES_PATTERN));

    let mut missing = "<schema/>".to_string();
    assert!(
        apply_pinned_regex_compatibility(&mut missing, PRODUCTION_SIMPLE_TYPES_SHA256).is_err()
    );
    assert_eq!(bounded("áβ界", 2), "áβ");

    let mut invalid_set = SchemaSet::new();
    let schema_error = invalid_set
        .add_document(Some("pinned://invalid.xsd"), "<not-xsd/>")
        .unwrap_err();
    let mapped = schema_assembly_error(schema_error);
    assert!(
        matches!(mapped, FiscalError::Artifact(message) if message.contains("assembly failed"))
    );
}

#[test]
fn diagnostics_do_not_echo_instance_values() {
    let xsd = concat!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema""#,
        r#" xmlns:t="http://www.sped.fazenda.gov.br/nfse""#,
        r#" targetNamespace="http://www.sped.fazenda.gov.br/nfse" elementFormDefault="qualified">"#,
        r#"<xs:simpleType name="CPF"><xs:restriction base="xs:string">"#,
        r#"<xs:pattern value="[0-9]{11}"/></xs:restriction></xs:simpleType>"#,
        r#"<xs:element name="DPS"><xs:complexType><xs:simpleContent>"#,
        r#"<xs:extension base="t:CPF"><xs:attribute name="versao" type="xs:string" use="required"/>"#,
        r#"</xs:extension></xs:simpleContent></xs:complexType></xs:element>"#,
        "</xs:schema>"
    );
    let mut set = SchemaSet::new();
    set.add_document(Some("pinned://DPS.xsd"), xsd).unwrap();
    let validator = NfseDpsSchemaValidator {
        schema: set.compile().unwrap(),
        profile: "synthetic-pattern",
    };
    assert!(validator.validate(&dps("52998224725")).is_ok());
    let error = validator.validate(&dps("529.982.247-25")).unwrap_err();
    let FiscalError::XmlValidation { message, .. } = &error else {
        panic!("expected a schema diagnostic");
    };
    assert!(message.contains("[redacted]"));
    assert!(!format!("{error} {error:?}").contains("529.982.247-25"));

    assert_eq!(
        redact_quoted_values("`a` then `b` and `open"),
        "`[redacted]` then `[redacted]` and `[redacted]`"
    );
    assert_eq!(redact_quoted_values("no quotes"), "no quotes");
}
