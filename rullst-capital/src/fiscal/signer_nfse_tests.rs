#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::fiscal::{
    FiscalCustomer, FiscalEmitter, IssRetention, IssTaxation, NFSE_PRODUCTION_V1_01_20260209,
    NFSE_RESTRICTED_V1_01_20260727, NfseArtifactManifest, NfseDpsSchemaValidator, NfseDpsV101,
    NfseEnvironment, TaxRegime, build_dps_xml_v1_01,
};
use chrono::{DateTime, NaiveDate};
use p12_keystore::{Certificate, KeyStoreEntry, PrivateKey, PrivateKeyChain};
use rcgen::{CertificateParams, KeyPair, PKCS_RSA_SHA256};
use xml_sec::xmldsig::{DefaultKeyResolver, DsigStatus, VerifyContext};

fn test_certificate() -> FiscalCertificate {
    let key_pair = KeyPair::generate_for(&PKCS_RSA_SHA256).unwrap();
    let certificate = CertificateParams::new(vec!["nfse.test".to_string()])
        .unwrap()
        .self_signed(&key_pair)
        .unwrap();
    let private_key = PrivateKey::from_der(key_pair.serialized_der()).unwrap();
    let certificate = Certificate::from_der(certificate.der().as_ref()).unwrap();
    let chain = PrivateKeyChain::new("nfse-test", private_key, [certificate]);
    let mut store = KeyStore::new();
    store.add_entry("nfse-test", KeyStoreEntry::PrivateKeyChain(chain));
    let pkcs12 = store.writer("test-passphrase").write().unwrap();
    FiscalCertificate::from_bytes(&pkcs12, "test-passphrase").unwrap()
}

fn unsigned_xml() -> &'static str {
    concat!(
        "<DPS xmlns=\"http://www.sped.fazenda.gov.br/nfse\" versao=\"1.01\">",
        "<infDPS Id=\"DPS355030821122233300018100001000000000000101\">",
        "<tpAmb>2</tpAmb></infDPS></DPS>"
    )
}

#[test]
fn signs_and_verifies_an_enveloped_rsa_sha256_document() {
    let certificate = test_certificate();
    let signed = sign_dps_xml(unsigned_xml(), &certificate).unwrap();

    assert!(signed.contains("http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"));
    assert!(signed.contains("http://www.w3.org/2001/04/xmlenc#sha256"));
    assert!(signed.contains("http://www.w3.org/TR/2001/REC-xml-c14n-20010315"));
    assert!(signed.contains("URI=\"#DPS355030821122233300018100001000000000000101\""));
    assert!(signed.contains("<X509Certificate>"));

    let resolver = DefaultKeyResolver::default();
    let verification = VerifyContext::new()
        .key_resolver(&resolver)
        .verify(&signed)
        .unwrap();
    assert_eq!(verification.status, DsigStatus::Valid);
    assert!(build_mtls_identity(&certificate).is_ok());
}

#[test]
fn refuses_existing_signatures_duplicate_ids_and_mock_credentials() {
    let duplicate = unsigned_xml().replace(
        "</infDPS>",
        "<extra Id=\"DPS355030821122233300018100001000000000000101\"/></infDPS>",
    );
    assert!(sign_dps_xml(&duplicate, &test_certificate()).is_err());

    let already_signed = unsigned_xml().replace(
        "</DPS>",
        "<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\"/></DPS>",
    );
    assert!(sign_dps_xml(&already_signed, &test_certificate()).is_err());
    assert!(sign_dps_xml(unsigned_xml(), &FiscalCertificate::offline_mock()).is_err());
}

#[test]
#[ignore = "requires an extracted checksum-pinned official NFS-e production XSD directory"]
fn signed_builder_output_matches_the_official_xsd_when_supplied() {
    let directory = std::env::var("RULLST_NFSE_XSD_DIR").unwrap();
    assert_signed_builder_schema(&directory, &NFSE_PRODUCTION_V1_01_20260209);
}

#[test]
#[ignore = "requires an extracted checksum-pinned official restricted-production XSD directory"]
fn signed_builder_output_matches_the_official_restricted_xsd_when_supplied() {
    let directory = std::env::var("RULLST_NFSE_RESTRICTED_XSD_DIR").unwrap();
    assert_signed_builder_schema(&directory, &NFSE_RESTRICTED_V1_01_20260727);
}

fn assert_signed_builder_schema(directory: &str, manifest: &'static NfseArtifactManifest) {
    let emitter = FiscalEmitter {
        cnpj: "11.222.333/0001-81".to_string(),
        inscricao_municipal: "12345".to_string(),
        legal_name: "Rullst Serviços Ltda".to_string(),
        trade_name: None,
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::SimplesNacional,
    };
    let customer = FiscalCustomer {
        doc_number: "529.982.247-25".to_string(),
        name: "Cliente Exemplo".to_string(),
        email: "fiscal@example.com".to_string(),
        zip_code: None,
        address: None,
        ibge_code: None,
    };
    let dps = NfseDpsV101 {
        id: "DPS355030821122233300018100001000000000000101".to_string(),
        series: "1".to_string(),
        number: 101,
        issued_at: DateTime::from_timestamp(1_767_268_800, 0).unwrap(),
        competence_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        service_code: "010301".to_string(),
        description: "Processamento de dados e SaaS".to_string(),
        amount_cents: 12_345,
        iss_rate_basis_points: Some(200),
        iss_taxation: IssTaxation::Taxable,
        iss_retention: IssRetention::NotRetained,
        service_city_ibge: "3550308".to_string(),
    };
    let unsigned =
        build_dps_xml_v1_01(&emitter, &customer, &dps, NfseEnvironment::Homologation).unwrap();
    let signed = sign_dps_xml(&unsigned, &test_certificate()).unwrap();
    let validator = NfseDpsSchemaValidator::from_pinned_directory(directory, manifest).unwrap();

    validator.validate(&signed).unwrap();
}
