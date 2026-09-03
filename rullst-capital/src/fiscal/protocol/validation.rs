use crate::fiscal::{
    MAX_DPS_XML_BYTES, MAX_SEFIN_RESPONSE_BYTES, NFSE_NAMESPACE, contract::XMLDSIG_NAMESPACE,
    models::FiscalError, signer::verify_embedded_xml_signature,
};

use super::{NfseApiEnvironment, invalid_dps, response_error};

pub(super) fn validate_signed_dps_shape(
    xml: &str,
) -> Result<(String, NfseApiEnvironment), FiscalError> {
    if xml.is_empty() || xml.len() > MAX_DPS_XML_BYTES || xml.contains("<!DOCTYPE") {
        return Err(invalid_dps(
            "signed DPS is empty, oversized or contains DOCTYPE",
        ));
    }
    let document = roxmltree::Document::parse(xml)
        .map_err(|_| invalid_dps("signed DPS is not well-formed XML"))?;
    let root = document.root_element();
    if root.tag_name().name() != "DPS"
        || root.tag_name().namespace() != Some(NFSE_NAMESPACE)
        || root.attribute("versao") != Some("1.01")
    {
        return Err(invalid_dps(
            "expected a DPS 1.01 root in the official namespace",
        ));
    }
    let information = root
        .children()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "infDPS"
                && node.tag_name().namespace() == Some(NFSE_NAMESPACE)
        })
        .collect::<Vec<_>>();
    if information.len() != 1 {
        return Err(invalid_dps("signed DPS must contain one direct infDPS"));
    }
    let dps_id = information[0]
        .attribute("Id")
        .ok_or_else(|| invalid_dps("signed DPS infDPS must carry an Id"))?;
    if dps_id.len() != 45
        || !dps_id.starts_with("DPS")
        || !dps_id[3..].bytes().all(|byte| byte.is_ascii_digit())
        || document
            .descendants()
            .filter(|node| node.attribute("Id") == Some(dps_id))
            .count()
            != 1
    {
        return Err(invalid_dps(
            "infDPS must carry one unique official 45-character Id",
        ));
    }
    validate_signature_binding(&document, root, dps_id)
        .map_err(|_| invalid_dps("XMLDSig must uniquely reference the direct infDPS child"))?;
    verify_embedded_xml_signature(xml)
        .map_err(|_| invalid_dps("embedded DPS XMLDSig did not verify"))?;
    let environment = signed_environment(information[0])?;
    Ok((dps_id.to_string(), environment))
}

fn signed_environment(
    information: roxmltree::Node<'_, '_>,
) -> Result<NfseApiEnvironment, FiscalError> {
    let values = information
        .children()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "tpAmb"
                && node.tag_name().namespace() == Some(NFSE_NAMESPACE)
        })
        .collect::<Vec<_>>();
    if values.len() != 1 {
        return Err(invalid_dps("infDPS must contain one direct tpAmb"));
    }
    match values[0].text() {
        Some("1") => Ok(NfseApiEnvironment::Production),
        Some("2") => Ok(NfseApiEnvironment::Homologation),
        _ => Err(invalid_dps("signed tpAmb must be 1 or 2")),
    }
}

pub(super) fn validate_authorized_nfse(xml: &str, access_key: &str) -> Result<(), FiscalError> {
    if xml.is_empty() || xml.len() > MAX_SEFIN_RESPONSE_BYTES || xml.contains("<!DOCTYPE") {
        return Err(response_error(
            "authorized NFS-e XML is empty, oversized or unsafe",
        ));
    }
    let document = roxmltree::Document::parse(xml)
        .map_err(|_| response_error("authorized NFS-e is not well-formed XML"))?;
    let root = document.root_element();
    if root.tag_name().name() != "NFSe"
        || root.tag_name().namespace() != Some(NFSE_NAMESPACE)
        || root.attribute("versao") != Some("1.01")
    {
        return Err(response_error(
            "authorized XML is not an NFS-e 1.01 document",
        ));
    }
    let expected_id = format!("NFS{access_key}");
    let mut information = root.children().filter(|node| {
        node.is_element()
            && node.tag_name().name() == "infNFSe"
            && node.tag_name().namespace() == Some(NFSE_NAMESPACE)
    });
    let first_information = information.next();
    let has_extra_information = information.next().is_some();
    if first_information.and_then(|node| node.attribute("Id")) != Some(expected_id.as_str())
        || has_extra_information
    {
        return Err(response_error(
            "infNFSe Id does not bind the returned access key",
        ));
    }
    validate_signature_binding(&document, root, &expected_id)?;
    verify_embedded_xml_signature(xml)
        .map_err(|_| response_error("authorized NFS-e XMLDSig did not verify"))?;
    Ok(())
}

fn validate_signature_binding(
    document: &roxmltree::Document<'_>,
    root: roxmltree::Node<'_, '_>,
    target_id: &str,
) -> Result<(), FiscalError> {
    let direct_signatures = root
        .children()
        .filter(|node| is_xml_signature(*node))
        .collect::<Vec<_>>();
    if direct_signatures.len() != 1
        || document
            .descendants()
            .filter(|node| is_xml_signature(*node))
            .count()
            != 1
    {
        return Err(response_error(
            "XML must contain one direct XMLDSig Signature",
        ));
    }
    let expected_reference = format!("#{target_id}");
    let references = direct_signatures[0]
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "Reference"
                && node.tag_name().namespace() == Some(XMLDSIG_NAMESPACE)
        })
        .collect::<Vec<_>>();
    if references.len() != 1 || references[0].attribute("URI") != Some(expected_reference.as_str())
    {
        return Err(response_error(
            "XMLDSig Reference does not bind the expected Id",
        ));
    }
    Ok(())
}

fn is_xml_signature(node: roxmltree::Node<'_, '_>) -> bool {
    node.is_element()
        && node.tag_name().name() == "Signature"
        && node.tag_name().namespace() == Some(XMLDSIG_NAMESPACE)
}
