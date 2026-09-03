use super::*;
use crate::fiscal::{NFSE_NAMESPACE, contract::XMLDSIG_NAMESPACE};
use rcgen::{CertificateParams, KeyPair, PKCS_RSA_SHA256};
use serde_json::json;
use std::sync::OnceLock;
use xml_sec::c14n::{C14nAlgorithm, C14nMode};
use xml_sec::xmldsig::{
    DigestAlgorithm, ReferenceBuilder, RsaSigningKey, SignContext, SignatureAlgorithm,
    SignatureBuilder, Transform, X509CertificateKeyInfoWriter,
};

const DPS_ID: &str = "DPS355030821122233300018100001000000000000101";
const ACCESS_KEY: &str = "35503082112223330001810000100000000000010112345678";

fn signed_dps() -> &'static str {
    static SIGNED_DPS: OnceLock<String> = OnceLock::new();
    SIGNED_DPS.get_or_init(|| {
        sign_fixture(
            &format!(
                "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{DPS_ID}\"><tpAmb>2</tpAmb></infDPS></DPS>"
            ),
            DPS_ID,
        )
    })
}

fn authorized_nfse() -> &'static str {
    static AUTHORIZED_NFSE: OnceLock<String> = OnceLock::new();
    AUTHORIZED_NFSE.get_or_init(|| {
        let id = format!("NFS{ACCESS_KEY}");
        sign_fixture(
            &format!(
                "<NFSe xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infNFSe Id=\"{id}\"><xLocEmi>São Paulo</xLocEmi></infNFSe></NFSe>"
            ),
            &id,
        )
    })
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
    STANDARD.encode(gzip(xml.as_bytes()).expect("GZip fixture"))
}

fn success_body() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "tipoAmbiente": 2,
        "versaoAplicativo": "SefinNacional_1.0",
        "dataHoraProcessamento": "2026-08-30T21:47:12.1202384-03:00",
        "idDps": DPS_ID,
        "chaveAcesso": ACCESS_KEY,
        "nfseXmlGZipB64": encoded_xml(authorized_nfse()),
        "alertas": [{"codigo": "A001", "descricao": "Fixture warning"}]
    }))
    .expect("JSON fixture")
}

#[test]
fn request_is_exact_deterministic_and_round_trips_the_signed_xml() {
    let request = NfseIssueRequest::try_from_signed_dps(signed_dps()).expect("request");
    let second = NfseIssueRequest::try_from_signed_dps(signed_dps()).expect("request");
    assert_eq!(request, second);
    assert_eq!(request.dps_id(), DPS_ID);
    assert_eq!(request.environment(), NfseApiEnvironment::Homologation);

    let value: serde_json::Value =
        serde_json::from_slice(&request.to_json().expect("request JSON")).expect("JSON value");
    assert_eq!(value.as_object().expect("object").len(), 1);
    let compressed = STANDARD
        .decode(
            value["dpsXmlGZipB64"]
                .as_str()
                .expect("base64 request field"),
        )
        .expect("base64");
    assert_eq!(
        gunzip_bounded(&compressed, MAX_DPS_XML_BYTES).expect("GZip DPS"),
        signed_dps()
    );
}

#[test]
fn request_rejects_unsigned_wrong_root_doctype_and_oversize() {
    assert!(NfseIssueRequest::try_from_signed_dps("<DPS/>").is_err());
    let decorative_signature = format!(
        "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{DPS_ID}\"/><Signature xmlns=\"{XMLDSIG_NAMESPACE}\"><SignedInfo><Reference URI=\"#{DPS_ID}\"/></SignedInfo></Signature></DPS>"
    );
    assert!(NfseIssueRequest::try_from_signed_dps(&decorative_signature).is_err());
    assert!(
        NfseIssueRequest::try_from_signed_dps(&format!(
            "<!DOCTYPE DPS><DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{DPS_ID}\"/><Signature xmlns=\"{XMLDSIG_NAMESPACE}\"/></DPS>"
        ))
        .is_err()
    );
    assert!(NfseIssueRequest::try_from_signed_dps(&"x".repeat(MAX_DPS_XML_BYTES + 1)).is_err());

    for environment_fields in [
        String::new(),
        "<tpAmb>3</tpAmb>".to_string(),
        "<tpAmb>2</tpAmb><tpAmb>2</tpAmb>".to_string(),
    ] {
        let invalid_environment = sign_fixture(
            &format!(
                "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{DPS_ID}\">{environment_fields}</infDPS></DPS>"
            ),
            DPS_ID,
        );
        assert!(NfseIssueRequest::try_from_signed_dps(&invalid_environment).is_err());
    }
}

#[test]
fn authorization_parser_binds_environment_dps_key_and_authorized_xml() {
    let request = NfseIssueRequest::try_from_signed_dps(signed_dps()).expect("request");
    let parsed = request
        .parse_response(201, NfseEnvironment::Homologation, &success_body())
        .expect("authorization response");
    let NfseIssueResponse::Authorized(authorization) = parsed else {
        panic!("expected authorization variant");
    };
    assert_eq!(authorization.environment, NfseApiEnvironment::Homologation);
    assert_eq!(authorization.dps_id, DPS_ID);
    assert_eq!(authorization.access_key, ACCESS_KEY);
    assert_eq!(authorization.authorized_xml, authorized_nfse());
    assert_eq!(authorization.warnings.len(), 1);
}

#[test]
fn rejection_parser_accepts_documented_case_variants_without_authorizing() {
    let request = NfseIssueRequest::try_from_signed_dps(signed_dps()).expect("request");
    let body = serde_json::to_vec(&json!({
        "tipoAmbiente": 2,
        "versaoAplicativo": "SefinNacional_1.0",
        "dataHoraProcessamento": "2026-08-30T21:47:12-03:00",
        "idDPS": DPS_ID,
        "erros": [{"Codigo": "E0712", "Descricao": "Rejected fixture"}]
    }))
    .expect("rejection JSON");
    let parsed = request
        .parse_response(400, NfseEnvironment::Homologation, &body)
        .expect("rejection response");
    let NfseIssueResponse::Rejected(rejection) = parsed else {
        panic!("expected rejection variant");
    };
    assert_eq!(rejection.http_status, 400);
    assert_eq!(rejection.errors[0].code.as_deref(), Some("E0712"));
}

#[test]
fn parser_rejects_mismatch_unknown_fields_invalid_status_and_empty_errors() {
    let request = NfseIssueRequest::try_from_signed_dps(signed_dps()).expect("request");
    assert!(
        request
            .parse_response(201, NfseEnvironment::Production, &success_body())
            .is_err()
    );
    assert!(
        request
            .parse_response(200, NfseEnvironment::Homologation, &success_body())
            .is_err()
    );

    let mut unknown: serde_json::Value =
        serde_json::from_slice(&success_body()).expect("success JSON");
    unknown["unexpected"] = json!(true);
    assert!(
        request
            .parse_response(
                201,
                NfseEnvironment::Homologation,
                &serde_json::to_vec(&unknown).expect("JSON"),
            )
            .is_err()
    );

    let rejection = serde_json::to_vec(&json!({
        "tipoAmbiente": 2,
        "versaoAplicativo": "SefinNacional_1.0",
        "dataHoraProcessamento": "2026-08-30T21:47:12-03:00",
        "erros": []
    }))
    .expect("JSON");
    assert!(
        request
            .parse_response(400, NfseEnvironment::Homologation, &rejection)
            .is_err()
    );
}

#[test]
// TM-PAY-07: confused, tampered, malformed or amplified responses must never authorize.
fn parser_rejects_confused_tampered_malformed_and_amplified_responses() {
    let request = NfseIssueRequest::try_from_signed_dps(signed_dps()).expect("request");
    assert!(
        request
            .parse_response(201, NfseEnvironment::Production, &success_body())
            .is_err()
    );

    let mut unknown: serde_json::Value =
        serde_json::from_slice(&success_body()).expect("success JSON");
    unknown["unexpected"] = json!(true);
    assert!(
        request
            .parse_response(
                201,
                NfseEnvironment::Homologation,
                &serde_json::to_vec(&unknown).expect("JSON"),
            )
            .is_err()
    );

    for (field, value) in [
        ("idDps", json!("DPS-other")),
        ("chaveAcesso", json!("123")),
        ("nfseXmlGZipB64", json!("not-base64")),
        (
            "nfseXmlGZipB64",
            json!(encoded_xml(&format!(
                "<NFSe xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infNFSe Id=\"NFS{}\"/></NFSe>",
                "0".repeat(ACCESS_KEY_BYTES)
            ))),
        ),
    ] {
        let mut body: serde_json::Value =
            serde_json::from_slice(&success_body()).expect("success JSON");
        body[field] = value;
        assert!(
            request
                .parse_response(
                    201,
                    NfseEnvironment::Homologation,
                    &serde_json::to_vec(&body).expect("JSON"),
                )
                .is_err(),
            "field {field} must fail"
        );
    }

    let oversized_xml = "x".repeat(MAX_SEFIN_RESPONSE_BYTES + 1);
    let mut body: serde_json::Value =
        serde_json::from_slice(&success_body()).expect("success JSON");
    body["nfseXmlGZipB64"] = json!(encoded_xml(&oversized_xml));
    assert!(
        request
            .parse_response(
                201,
                NfseEnvironment::Homologation,
                &serde_json::to_vec(&body).expect("JSON"),
            )
            .is_err()
    );

    let mut tampered_body: serde_json::Value =
        serde_json::from_slice(&success_body()).expect("success JSON");
    tampered_body["nfseXmlGZipB64"] = json!(encoded_xml(
        &authorized_nfse().replace("São Paulo", "Rio de Janeiro")
    ));
    assert!(
        request
            .parse_response(
                201,
                NfseEnvironment::Homologation,
                &serde_json::to_vec(&tampered_body).expect("JSON"),
            )
            .is_err()
    );
}
