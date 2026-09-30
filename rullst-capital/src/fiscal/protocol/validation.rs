use crate::fiscal::{
    MAX_DPS_XML_BYTES, MAX_SEFIN_RESPONSE_BYTES, NFSE_NAMESPACE,
    contract::XMLDSIG_NAMESPACE,
    models::FiscalError,
    signer::{verify_embedded_xml_signature, verify_first_xml_signature},
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

pub(super) fn validate_authorized_nfse(
    xml: &str,
    access_key: &str,
    expected_dps_id: &str,
    expected_environment: NfseApiEnvironment,
) -> Result<(), FiscalError> {
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
    let Some(information) = first_information.filter(|node| {
        node.attribute("Id") == Some(expected_id.as_str()) && !has_extra_information
    }) else {
        return Err(response_error(
            "infNFSe Id does not bind the returned access key",
        ));
    };
    if document
        .descendants()
        .filter(|node| node.attribute("Id") == Some(expected_id.as_str()))
        .count()
        != 1
    {
        return Err(response_error("infNFSe Id must be unique in the NFS-e"));
    }
    let signature = validate_nfse_signature_layout(&document, root, information, &expected_id)?;
    validate_embedded_dps(
        &document,
        information,
        expected_dps_id,
        expected_environment,
    )?;
    // SEFIN embeds the submitted signed DPS inside infNFSe, so a genuine
    // NFS-e has two signatures. Verify the authority's own root signature by
    // moving it to the front (sibling order does not change what it signs)
    // and selecting the first document signature explicitly.
    let verification_xml = root_signature_first(xml, root, signature)?;
    verify_first_xml_signature(&verification_xml)
        .map_err(|_| response_error("authorized NFS-e XMLDSig did not verify"))?;
    Ok(())
}

// Requires one authority signature as a direct child of the NFSe root that
// references infNFSe. The only other signature allowed is the embedded signed
// DPS's own, as a direct child of infNFSe/DPS; it is covered by the root
// signature's digest and is not verified separately.
fn validate_nfse_signature_layout<'a, 'input>(
    document: &'a roxmltree::Document<'input>,
    root: roxmltree::Node<'a, 'input>,
    information: roxmltree::Node<'a, 'input>,
    expected_id: &str,
) -> Result<roxmltree::Node<'a, 'input>, FiscalError> {
    let layout_error = || response_error("NFS-e must contain one root XMLDSig Signature");
    let mut direct = root.children().filter(|node| is_xml_signature(*node));
    let signature = direct.next().ok_or_else(layout_error)?;
    if direct.next().is_some() {
        return Err(layout_error());
    }
    let mut embedded = 0_usize;
    for node in document
        .descendants()
        .filter(|node| is_xml_signature(*node))
    {
        if node == signature {
            continue;
        }
        let parent = node.parent_element();
        let in_embedded_dps = parent.is_some_and(|dps| {
            dps.tag_name().name() == "DPS"
                && dps.tag_name().namespace() == Some(NFSE_NAMESPACE)
                && dps.parent_element() == Some(information)
        });
        embedded += 1;
        if !in_embedded_dps || embedded > 1 {
            return Err(response_error(
                "NFS-e XMLDSig is only allowed on the root and the embedded DPS",
            ));
        }
    }
    validate_reference(signature, expected_id)?;
    Ok(signature)
}

// Binds the authorization to this request inside the signed infNFSe rather
// than only through the unsigned JSON wrapper: the embedded DPS must carry the
// submitted Id and the requested tpAmb.
fn validate_embedded_dps(
    document: &roxmltree::Document<'_>,
    information: roxmltree::Node<'_, '_>,
    expected_dps_id: &str,
    expected_environment: NfseApiEnvironment,
) -> Result<(), FiscalError> {
    let binding_error = || response_error("signed NFS-e does not embed the submitted DPS");
    let dps = single_nfse_child(information, "DPS").ok_or_else(binding_error)?;
    let dps_information = single_nfse_child(dps, "infDPS").ok_or_else(binding_error)?;
    if dps_information.attribute("Id") != Some(expected_dps_id)
        || document
            .descendants()
            .filter(|node| node.attribute("Id") == Some(expected_dps_id))
            .count()
            != 1
    {
        return Err(binding_error());
    }
    if signed_environment(dps_information).map_err(|_| binding_error())? != expected_environment {
        return Err(response_error(
            "signed NFS-e DPS environment does not match the request",
        ));
    }
    Ok(())
}

fn single_nfse_child<'a, 'input>(
    parent: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    let mut children = parent.children().filter(|node| {
        node.is_element()
            && node.tag_name().name() == name
            && node.tag_name().namespace() == Some(NFSE_NAMESPACE)
    });
    let child = children.next()?;
    children.next().is_none().then_some(child)
}

fn root_signature_first(
    xml: &str,
    root: roxmltree::Node<'_, '_>,
    signature: roxmltree::Node<'_, '_>,
) -> Result<String, FiscalError> {
    let reorder_error = || response_error("cannot isolate the NFS-e root signature");
    let first = root
        .children()
        .find(|node| node.is_element())
        .ok_or_else(reorder_error)?;
    if first == signature {
        return Ok(xml.to_owned());
    }
    let insert_at = first.range().start;
    let moved = signature.range();
    if moved.start < insert_at {
        return Err(reorder_error());
    }
    let mut reordered = String::with_capacity(xml.len());
    for part in [
        xml.get(..insert_at),
        xml.get(moved.clone()),
        xml.get(insert_at..moved.start),
        xml.get(moved.end..),
    ] {
        reordered.push_str(part.ok_or_else(reorder_error)?);
    }
    Ok(reordered)
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
    validate_reference(direct_signatures[0], target_id)
}

fn validate_reference(
    signature: roxmltree::Node<'_, '_>,
    target_id: &str,
) -> Result<(), FiscalError> {
    let expected_reference = format!("#{target_id}");
    let references = signature
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
