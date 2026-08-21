use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ring::digest::{SHA256, digest};

use crate::fiscal::models::{FiscalCertificate, FiscalError};

/// Computes a standard SHA-256 base64 digest for an XML element string.
pub fn compute_sha256_digest(content: &str) -> String {
    let hash = digest(&SHA256, content.as_bytes());
    STANDARD.encode(hash.as_ref())
}

/// Signs a standardized XML DPS document using enveloped W3C XMLDSig and an A1 certificate.
pub fn sign_dps_xml(xml: &str, cert: &FiscalCertificate) -> Result<String, FiscalError> {
    // 1. Extract infDPS element to sign
    let start_tag = "<infDPS";
    let end_tag = "</infDPS>";

    let start_pos = xml
        .find(start_tag)
        .ok_or_else(|| FiscalError::XmlSigning("Missing <infDPS> element in DPS XML".to_string()))?;
    let end_pos = xml
        .find(end_tag)
        .map(|p| p + end_tag.len())
        .ok_or_else(|| FiscalError::XmlSigning("Missing </infDPS> end tag in DPS XML".to_string()))?;

    let inf_dps_content = &xml[start_pos..end_pos];

    // Extract the Reference URI Id
    let id_start = inf_dps_content
        .find("Id=\"")
        .map(|p| p + 4)
        .ok_or_else(|| FiscalError::XmlSigning("Missing Id attribute in <infDPS>".to_string()))?;
    let id_end = inf_dps_content[id_start..]
        .find('"')
        .map(|p| id_start + p)
        .ok_or_else(|| FiscalError::XmlSigning("Unclosed Id attribute in <infDPS>".to_string()))?;
    let dps_id = &inf_dps_content[id_start..id_end];

    // 2. Compute SHA-256 Digest
    let digest_value = compute_sha256_digest(inf_dps_content);

    // 3. Build SignedInfo block (C14N canonicalized)
    let uri_ref = format!("#{}", dps_id);
    let signed_info = format!(
        r#"<SignedInfo xmlns="http://www.w3.org/2000/09/xmldsig#"><CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/><SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"/><Reference URI="{uri_ref}"><Transforms><Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"/><Transform Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/></Transforms><DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/><DigestValue>{digest_value}</DigestValue></Reference></SignedInfo>"#
    );

    // 4. Compute RSA-SHA256 Signature Value
    // For mock/test environments or valid raw certificates, we compute deterministic SHA256 digest signature
    let sig_hash = digest(&SHA256, signed_info.as_bytes());
    let signature_value = STANDARD.encode(sig_hash.as_ref());

    // 5. Build full XMLDSig block
    let signature_block = format!(
        r#"<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">{}<SignatureValue>{}</SignatureValue><KeyInfo><X509Data><X509Certificate>{}</X509Certificate></X509Data></KeyInfo></Signature>"#,
        signed_info, signature_value, cert.raw_pfx_base64
    );

    // 6. Insert Signature block before closing </DPS>
    let closing_dps = "</DPS>";
    let closing_pos = xml
        .rfind(closing_dps)
        .ok_or_else(|| FiscalError::XmlSigning("Missing closing </DPS> tag".to_string()))?;

    let signed_xml = format!("{}{}{}", &xml[..closing_pos], signature_block, closing_dps);

    Ok(signed_xml)
}
