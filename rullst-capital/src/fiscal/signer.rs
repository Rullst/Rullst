use base64::Engine;
use base64::engine::general_purpose::STANDARD;
#[cfg(feature = "nfse")]
use p12_keystore::{KeyStore, Pkcs12ImportPolicy};
use ring::digest::{SHA256, digest};
#[cfg(feature = "nfse")]
use xml_sec::c14n::{C14nAlgorithm, C14nMode};
#[cfg(feature = "nfse")]
use xml_sec::xmldsig::{
    DefaultKeyResolver, DigestAlgorithm, DsigStatus, ReferenceBuilder, RsaSigningKey, SignContext,
    SignatureAlgorithm, SignatureBuilder, Transform, VerifyContext, X509CertificateKeyInfoWriter,
};
#[cfg(feature = "nfse")]
use zeroize::Zeroizing;

#[cfg(feature = "nfse")]
use crate::fiscal::contract::{MAX_DPS_XML_BYTES, NFSE_NAMESPACE, XMLDSIG_NAMESPACE};
use crate::fiscal::models::{FiscalCertificate, FiscalError};

/// Computes a standard SHA-256 base64 digest for an XML element string.
pub fn compute_sha256_digest(content: &str) -> String {
    let hash = digest(&SHA256, content.as_bytes());
    STANDARD.encode(hash.as_ref())
}

#[cfg(feature = "nfse")]
/// Signs one bounded DPS 1.01 document using its `infDPS/@Id` and an A1 PKCS#12 keypair.
///
/// The generated enveloped signature uses inclusive C14N 1.0, SHA-256 and RSA-SHA256. This local
/// cryptographic operation does not imply SEFIN authorization or ICP-Brasil chain validation.
pub fn sign_dps_xml(xml: &str, certificate: &FiscalCertificate) -> Result<String, FiscalError> {
    let dps_id = validate_unsigned_dps_envelope(xml)?;
    let store = certificate_store(certificate)?;
    let (_, key_chain) = store.private_key_chain().ok_or_else(|| {
        FiscalError::Certificate(
            "PKCS#12 does not contain a private key with a certificate chain".to_string(),
        )
    })?;
    if key_chain.certs().is_empty() {
        return Err(FiscalError::Certificate(
            "PKCS#12 private key has no matching X.509 certificate".to_string(),
        ));
    }

    let signing_key = RsaSigningKey::from_pkcs8_der(key_chain.key().as_der()).map_err(|_| {
        FiscalError::Certificate(
            "PKCS#12 private key is not a supported RSA PKCS#8 key".to_string(),
        )
    })?;
    let key_info = X509CertificateKeyInfoWriter::from_der_chain(
        key_chain.certs().iter().map(|entry| entry.as_der()),
    )
    .map_err(|_| {
        FiscalError::Certificate("PKCS#12 contains an invalid X.509 certificate chain".to_string())
    })?;

    let c14n = C14nAlgorithm::new(C14nMode::Inclusive1_0, false);
    let template = SignatureBuilder::new(c14n.clone(), SignatureAlgorithm::RsaSha256)
        .add_reference(
            ReferenceBuilder::new(DigestAlgorithm::Sha256)
                .uri(format!("#{dps_id}"))
                .transform(Transform::Enveloped)
                .transform(Transform::C14n(c14n)),
        )
        .key_info(true)
        .build_template()
        .map_err(|error| signing_error("cannot build XMLDSig template", &error))?;
    let with_template = xml_sec::xmldsig::mutation::append_signature_to_root(xml, &template)
        .map_err(|error| signing_error("cannot append XMLDSig template", &error))?;
    let signed = SignContext::new(&signing_key)
        .key_info_writer(&key_info)
        .sign_template(&with_template)
        .map_err(|error| signing_error("cannot sign DPS XML", &error))?;
    if signed.len() > MAX_DPS_XML_BYTES {
        return Err(FiscalError::XmlSigning(
            "signed DPS exceeds the one MiB limit".to_string(),
        ));
    }
    verify_embedded_xml_signature(&signed)?;
    Ok(signed)
}

#[cfg(feature = "nfse")]
pub(crate) fn verify_embedded_xml_signature(xml: &str) -> Result<(), FiscalError> {
    let resolver = DefaultKeyResolver::default();
    let verification = VerifyContext::new()
        .key_resolver(&resolver)
        .verify(xml)
        .map_err(|error| signing_error("cannot verify embedded XMLDSig", &error))?;
    if verification.status != DsigStatus::Valid {
        return Err(FiscalError::XmlSigning(
            "embedded XMLDSig did not verify with its declared certificate".to_string(),
        ));
    }
    Ok(())
}

#[cfg(not(feature = "nfse"))]
/// Fails closed until the opt-in `nfse` feature enables the audited signing dependencies.
pub fn sign_dps_xml(_xml: &str, _certificate: &FiscalCertificate) -> Result<String, FiscalError> {
    Err(FiscalError::Unsupported(
        "NFS-e signing requires the `rullst-capital/nfse` feature".to_string(),
    ))
}

#[cfg(feature = "nfse")]
pub(crate) fn build_mtls_identity(
    certificate: &FiscalCertificate,
) -> Result<reqwest::Identity, FiscalError> {
    let store = certificate_store(certificate)?;
    let (_, key_chain) = store.private_key_chain().ok_or_else(|| {
        FiscalError::Certificate(
            "PKCS#12 does not contain a private key with a certificate chain".to_string(),
        )
    })?;
    if key_chain.certs().is_empty() {
        return Err(FiscalError::Certificate(
            "PKCS#12 private key has no matching X.509 certificate".to_string(),
        ));
    }

    let mut pem = Zeroizing::new(String::new());
    append_pem_block(&mut pem, "PRIVATE KEY", key_chain.key().as_der());
    for certificate in key_chain.certs() {
        append_pem_block(&mut pem, "CERTIFICATE", certificate.as_der());
    }
    reqwest::Identity::from_pem(pem.as_bytes()).map_err(|_| {
        FiscalError::Certificate(
            "PKCS#12 key and certificate chain cannot form a rustls mTLS identity".to_string(),
        )
    })
}

#[cfg(feature = "nfse")]
fn certificate_store(certificate: &FiscalCertificate) -> Result<KeyStore, FiscalError> {
    KeyStore::from_pkcs12(
        certificate.pkcs12_der()?,
        certificate.passphrase()?,
        Pkcs12ImportPolicy::Strict,
    )
    .map_err(|_| {
        FiscalError::Certificate(
            "cannot open PKCS#12 certificate, passphrase or key chain".to_string(),
        )
    })
}

#[cfg(feature = "nfse")]
fn append_pem_block(output: &mut String, label: &str, der: &[u8]) {
    output.push_str("-----BEGIN ");
    output.push_str(label);
    output.push_str("-----\n");
    let encoded = STANDARD.encode(der);
    for chunk in encoded.as_bytes().chunks(64) {
        output.extend(chunk.iter().map(|byte| char::from(*byte)));
        output.push('\n');
    }
    output.push_str("-----END ");
    output.push_str(label);
    output.push_str("-----\n");
}

#[cfg(feature = "nfse")]
fn validate_unsigned_dps_envelope(xml: &str) -> Result<String, FiscalError> {
    if xml.is_empty() || xml.len() > MAX_DPS_XML_BYTES {
        return Err(FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "unsigned DPS must contain between one byte and one MiB".to_string(),
        });
    }
    if xml.contains("<!DOCTYPE") {
        return Err(FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "DOCTYPE is forbidden in fiscal XML".to_string(),
        });
    }
    let document = roxmltree::Document::parse(xml).map_err(|_| FiscalError::InvalidInput {
        field: "dps.xml",
        reason: "document is not well-formed XML".to_string(),
    })?;
    let root = document.root_element();
    if root.tag_name().name() != "DPS"
        || root.tag_name().namespace() != Some(NFSE_NAMESPACE)
        || root.attribute("versao") != Some("1.01")
    {
        return Err(FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "expected a DPS 1.01 root in the official NFS-e namespace".to_string(),
        });
    }
    if document.descendants().any(|node| {
        node.is_element()
            && node.tag_name().name() == "Signature"
            && node.tag_name().namespace() == Some(XMLDSIG_NAMESPACE)
    }) {
        return Err(FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "document already contains an XMLDSig Signature".to_string(),
        });
    }
    let mut information_nodes = root.children().filter(|node| {
        node.is_element()
            && node.tag_name().name() == "infDPS"
            && node.tag_name().namespace() == Some(NFSE_NAMESPACE)
    });
    let information = information_nodes
        .next()
        .ok_or_else(|| FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "DPS root must contain exactly one direct infDPS child".to_string(),
        })?;
    if information_nodes.next().is_some() {
        return Err(FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "DPS root must contain exactly one direct infDPS child".to_string(),
        });
    }
    let id = information
        .attribute("Id")
        .filter(|value| {
            value.len() == 45
                && value.starts_with("DPS")
                && value[3..].bytes().all(|byte| byte.is_ascii_digit())
        })
        .ok_or_else(|| FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "infDPS must carry an official 45-character Id".to_string(),
        })?;
    let duplicate_id = document
        .descendants()
        .filter(|node| node.attribute("Id") == Some(id))
        .count();
    if duplicate_id != 1 {
        return Err(FiscalError::InvalidInput {
            field: "dps.xml",
            reason: "infDPS Id must be unique in the document".to_string(),
        });
    }
    Ok(id.to_string())
}

#[cfg(feature = "nfse")]
fn signing_error(context: &str, error: &dyn std::fmt::Display) -> FiscalError {
    let diagnostic = error.to_string().chars().take(384).collect::<String>();
    FiscalError::XmlSigning(format!("{context}: {diagnostic}"))
}

#[cfg(all(test, feature = "nfse"))]
#[path = "signer_nfse_tests.rs"]
mod nfse_tests;

#[cfg(test)]
#[path = "signer_contract_tests.rs"]
mod contract_tests;
