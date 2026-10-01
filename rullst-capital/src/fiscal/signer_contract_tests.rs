#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[cfg(feature = "nfse")]
const DPS_ID: &str = "DPS355030821122233300018100001000000000000101";

#[test]
fn sha256_digest_matches_the_standard_vector() {
    assert_eq!(
        compute_sha256_digest("hello"),
        "LPJNul+wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ="
    );
}

#[cfg(not(feature = "nfse"))]
#[test]
fn signing_fails_closed_without_the_nfse_feature() {
    assert!(matches!(
        sign_dps_xml("<DPS/>", &FiscalCertificate::offline_mock()),
        Err(FiscalError::Unsupported(message)) if message.contains("nfse")
    ));
}

#[cfg(feature = "nfse")]
fn unsigned_xml() -> String {
    format!(
        "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{DPS_ID}\"><tpAmb>2</tpAmb></infDPS></DPS>"
    )
}

#[cfg(feature = "nfse")]
fn assert_invalid_envelope(xml: &str, expected_reason: &str) {
    assert!(matches!(
        validate_unsigned_dps_envelope(xml),
        Err(FiscalError::InvalidInput {
            field: "dps.xml",
            reason,
        }) if reason.contains(expected_reason)
    ));
}

#[cfg(feature = "nfse")]
#[test]
fn unsigned_envelope_validation_rejects_untrusted_shapes_and_accepts_one_official_id() {
    assert_invalid_envelope("", "between one byte");
    assert_invalid_envelope(&"x".repeat(MAX_DPS_XML_BYTES + 1), "between one byte");
    assert_invalid_envelope(
        "<!DOCTYPE DPS><DPS xmlns=\"http://www.sped.fazenda.gov.br/nfse\" versao=\"1.01\"/>",
        "DOCTYPE",
    );
    assert_invalid_envelope("<DPS", "well-formed");
    assert_invalid_envelope(
        &format!("<Other xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"/>"),
        "expected a DPS 1.01 root",
    );
    assert_invalid_envelope(
        &format!("<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.00\"/>"),
        "expected a DPS 1.01 root",
    );
    assert_invalid_envelope(
        &format!(
            "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><Signature xmlns=\"{XMLDSIG_NAMESPACE}\"/></DPS>"
        ),
        "already contains",
    );
    assert_invalid_envelope(
        &format!("<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"/>"),
        "exactly one",
    );
    assert_invalid_envelope(
        &format!(
            "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{DPS_ID}\"/><infDPS Id=\"DPS355030821122233300018100001000000000000102\"/></DPS>"
        ),
        "exactly one",
    );
    assert_invalid_envelope(
        &format!("<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS/></DPS>"),
        "official 45-character Id",
    );
    assert_invalid_envelope(
        &format!(
            "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"DPS-not-numeric\"/></DPS>"
        ),
        "official 45-character Id",
    );
    assert_invalid_envelope(
        &unsigned_xml().replace("</infDPS>", &format!("<nested Id=\"{DPS_ID}\"/></infDPS>")),
        "must be unique",
    );

    assert_eq!(
        validate_unsigned_dps_envelope(&unsigned_xml()).unwrap(),
        DPS_ID
    );
}

#[cfg(feature = "nfse")]
#[test]
fn invalid_certificate_signature_and_pem_helpers_fail_closed_and_remain_bounded() {
    let invalid = FiscalCertificate::from_bytes(b"not-a-pkcs12-container", "wrong-pass").unwrap();
    assert!(matches!(
        sign_dps_xml(&unsigned_xml(), &invalid),
        Err(FiscalError::Certificate(message)) if message.contains("cannot open")
    ));
    assert!(matches!(
        build_mtls_identity(&invalid),
        Err(FiscalError::Certificate(message)) if message.contains("cannot open")
    ));
    assert!(matches!(
        verify_embedded_xml_signature(&unsigned_xml()),
        Err(FiscalError::XmlSigning(message)) if message.contains("verify")
    ));

    let mut pem = String::new();
    append_pem_block(&mut pem, "FIXTURE", &[7; 65]).unwrap();
    let lines = pem.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "-----BEGIN FIXTURE-----");
    assert_eq!(lines[1].len(), 64);
    assert_eq!(lines[2].len(), 24);
    assert_eq!(lines[3], "-----END FIXTURE-----");

    // The identity buffer is sized exactly, so writing key material never
    // reallocates and leaves an unzeroized copy behind.
    for sizes in [&[65_usize, 1_200][..], &[48], &[1, 2, 3, 4_096]] {
        let ders = sizes
            .iter()
            .map(|size| vec![9_u8; *size])
            .collect::<Vec<_>>();
        let blocks = ders
            .iter()
            .enumerate()
            .map(|(index, der)| {
                let label = if index == 0 {
                    "PRIVATE KEY"
                } else {
                    "CERTIFICATE"
                };
                (label, der.as_slice())
            })
            .collect::<Vec<_>>();
        let pem = encode_pem_blocks(&blocks).unwrap();
        assert_eq!(pem.len(), pem.capacity());
        assert_eq!(pem.matches("-----BEGIN ").count(), sizes.len());
    }

    let mapped = signing_error("bounded", &"x".repeat(1_000));
    assert!(matches!(
        mapped,
        FiscalError::XmlSigning(message)
            if message.starts_with("bounded: ") && message.len() == "bounded: ".len() + 384
    ));
}
