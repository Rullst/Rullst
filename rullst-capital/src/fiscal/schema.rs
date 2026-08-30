//! Offline-only validation against checksum-pinned official NFS-e XSD bundles.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use oxixml_schema::{ResolvedSchema, SchemaResolver, SchemaSet};
use ring::digest::{SHA256, digest};

use crate::fiscal::contract::{
    MAX_DPS_XML_BYTES, NFSE_PRODUCTION_V1_01_20260209, NFSE_RESTRICTED_V1_01_20260727,
    NfseArtifactManifest,
};
use crate::fiscal::models::FiscalError;

const MAX_XSD_FILE_BYTES: u64 = 256 * 1024;
const PRODUCTION_SIMPLE_TYPES_SHA256: &str =
    "830ea116c34d7310699e34b214b7214a65f7e5d3b1f09aeaa702f7e3f4283b17";
const OFFICIAL_SERIES_PATTERN: &str = "value=\"^0{0,4}\\d{1,5}$\"";
const XSD_COMPATIBLE_SERIES_PATTERN: &str = "value=\"0{0,4}\\d{1,5}\"";

const PRODUCTION_FILES: [(&str, &str); 10] = [
    (
        "CNC_v1.00.xsd",
        "7032188bb6f137d52b16512583b739a361e6b1434c42cb539a29b8efa32f8321",
    ),
    (
        "DPS_v1.01.xsd",
        "fe45e5250a48e519aba89fc6a472863b8e602ed957778fb64692804933a00d0c",
    ),
    (
        "NFSe_v1.01.xsd",
        "af0bd2d8c50acba3d9c3f3f515426eca083b3f66e3211ce8dd48a7cf101818b8",
    ),
    (
        "evento_v1.01.xsd",
        "986d0a1c4d27454f712169849aa7c2380aaa4560bd9c920e0ff03ba6599ae28b",
    ),
    (
        "pedRegEvento_v1.01.xsd",
        "e90b6816d29cca0bd5ed8f86ad98d3b7b0d8bbfc11a7583e6ff8f98170fd4009",
    ),
    (
        "tiposCnc_v1.00.xsd",
        "af606b7317824fa8fa7ad8e44bac2ad8530cd5d162ebcdec647205473c41d525",
    ),
    (
        "tiposComplexos_v1.01.xsd",
        "e8e09d525574cc224ca6d1f8d8eb0366043ab2a3aa8d0d234058e6244d60e371",
    ),
    (
        "tiposEventos_v1.01.xsd",
        "1b32bea21089dc232d78b62d805e9c07382f440d4e0ea2cb3fb6b82ebde68c11",
    ),
    (
        "tiposSimples_v1.01.xsd",
        "830ea116c34d7310699e34b214b7214a65f7e5d3b1f09aeaa702f7e3f4283b17",
    ),
    (
        "xmldsig-core-schema.xsd",
        "49848f732663aecb618d72ad6130c5c3240f0a10f3a1a8544b7d48a6c726046f",
    ),
];

const RESTRICTED_FILES: [(&str, &str); 10] = [
    (
        "CNC_v1.00.xsd",
        "26961f705970ccb1dede1b6bb0ab544ca1d089137c22e93371d789c1e15ef0c4",
    ),
    (
        "DPS_v1.01.xsd",
        "c7dab363d8cf7c83fc2b3b21e72cf669a51bd30947a5690685ea96c4b3e39dcd",
    ),
    (
        "NFSe_v1.01.xsd",
        "1dd8f543060a4ba6f355693f1fa5d79a269acae7c3dfe42d621f62911cfebac0",
    ),
    (
        "evento_v1.01.xsd",
        "dd14062174d439a67a11266d82e822cb9021db1595a41bd876a16560a38f6aec",
    ),
    (
        "pedRegEvento_v1.01.xsd",
        "186c83f33752a195845300af61ffbe7136547f84cab032b35238c78f9d78f7c6",
    ),
    (
        "tiposCnc_v1.00.xsd",
        "c1cc33f1007251075b1fff7766bf881bba6a8ad7f1f36d149e54ae98090c77f8",
    ),
    (
        "tiposComplexos_v1.01.xsd",
        "6f792f408a33c11e799042a8d61cac7d1c9f5992c53e07e60ce75a15f157d1ac",
    ),
    (
        "tiposEventos_v1.01.xsd",
        "6c9ae744b1cb886607c1138c32eeb76cd410b856ce7301197b95d48d63f7b40b",
    ),
    (
        "tiposSimples_v1.01.xsd",
        "3d8171c9b7c9a82ecb48eed9a96485f2077006e7d21db6cd182839dd34dbb5e4",
    ),
    (
        "xmldsig-core-schema.xsd",
        "bf43998b2df1fedd9ed7d6914f91ab4d34958e8730c3b500cbe0b21e60335f11",
    ),
];

/// Compiled DPS schema whose source files matched a recognized official manifest exactly.
///
/// The pinned production simple-types source then receives one exact regex compatibility
/// normalization documented by the module; any different source or occurrence count fails closed.
pub struct NfseDpsSchemaValidator {
    schema: oxixml_schema::Schema,
    profile: &'static str,
}

impl NfseDpsSchemaValidator {
    /// Loads, hashes and compiles a recognized extracted official schema directory.
    ///
    /// All resolution happens from a bounded in-memory catalogue. The XML instance cannot make
    /// this validator follow `schemaLocation` hints or access the filesystem/network.
    pub fn from_pinned_directory(
        directory: impl AsRef<Path>,
        manifest: &'static NfseArtifactManifest,
    ) -> Result<Self, FiscalError> {
        let expected_files = expected_files(manifest)?;
        let documents = load_verified_documents(directory.as_ref(), expected_files)?;
        let root = documents
            .get(manifest.dps_root_schema)
            .ok_or_else(|| FiscalError::Artifact("pinned DPS root schema is missing".to_string()))?
            .clone();
        let resolver = PinnedResolver {
            documents: documents.clone(),
        };
        let mut set = SchemaSet::new().with_resolver(Box::new(resolver));
        set.add_document(
            Some(&format!("pinned://{}", manifest.dps_root_schema)),
            &root,
        )
        .map_err(schema_assembly_error)?;
        let schema = set.compile().map_err(schema_assembly_error)?;
        Ok(Self {
            schema,
            profile: manifest.profile,
        })
    }

    /// Stable manifest profile used to compile this validator.
    #[must_use]
    pub fn profile(&self) -> &'static str {
        self.profile
    }

    /// Validates a bounded DPS XML without following instance-provided hints.
    pub fn validate(&self, xml: &str) -> Result<(), FiscalError> {
        if xml.is_empty() || xml.len() > MAX_DPS_XML_BYTES {
            return Err(FiscalError::InvalidInput {
                field: "dps.xml",
                reason: "document must contain between one byte and one MiB".to_string(),
            });
        }
        if xml.contains("<!DOCTYPE") {
            return Err(FiscalError::InvalidInput {
                field: "dps.xml",
                reason: "DOCTYPE is forbidden in fiscal XML".to_string(),
            });
        }
        let outcome = self.schema.validate_str(xml);
        if outcome.valid {
            return Ok(());
        }
        let Some(error) = outcome.first_error() else {
            return Err(FiscalError::XmlValidation {
                code: "unknown".to_string(),
                path: "/".to_string(),
                message: "schema validation failed without a diagnostic".to_string(),
            });
        };
        Err(FiscalError::XmlValidation {
            code: bounded(error.code(), 96),
            path: bounded(error.path(), 256),
            message: bounded(error.message(), 512),
        })
    }
}

#[derive(Debug, Clone)]
struct PinnedResolver {
    documents: BTreeMap<String, String>,
}

impl SchemaResolver for PinnedResolver {
    fn resolve(
        &mut self,
        location: &str,
        _base: Option<&str>,
    ) -> Result<Option<ResolvedSchema>, oxixml_schema::SchemaError> {
        if location.contains('/') || location.contains('\\') || location.contains("..") {
            return Ok(None);
        }
        Ok(self.documents.get(location).map(|text| ResolvedSchema {
            uri: format!("pinned://{location}"),
            text: text.clone(),
        }))
    }
}

fn expected_files(
    manifest: &NfseArtifactManifest,
) -> Result<&'static [(&'static str, &'static str)], FiscalError> {
    if *manifest == NFSE_PRODUCTION_V1_01_20260209 {
        return Ok(&PRODUCTION_FILES);
    }
    if *manifest == NFSE_RESTRICTED_V1_01_20260727 {
        return Ok(&RESTRICTED_FILES);
    }
    Err(FiscalError::Artifact(
        "unrecognized NFS-e artifact manifest".to_string(),
    ))
}

fn load_verified_documents(
    directory: &Path,
    expected_files: &[(&str, &str)],
) -> Result<BTreeMap<String, String>, FiscalError> {
    let mut documents = BTreeMap::new();
    for (file_name, expected_hash) in expected_files {
        let path = directory.join(file_name);
        let file = std::fs::File::open(&path).map_err(|_| {
            FiscalError::Artifact(format!("required schema `{file_name}` is missing"))
        })?;
        let mut bytes = Vec::new();
        file.take(MAX_XSD_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| FiscalError::Artifact(format!("cannot read schema `{file_name}`")))?;
        if bytes.len() as u64 > MAX_XSD_FILE_BYTES {
            return Err(FiscalError::Artifact(format!(
                "schema `{file_name}` exceeds the 256 KiB limit"
            )));
        }
        let actual_hash = hex::encode(digest(&SHA256, &bytes).as_ref());
        if actual_hash != *expected_hash {
            return Err(FiscalError::Artifact(format!(
                "schema `{file_name}` does not match the pinned SHA-256"
            )));
        }
        let mut text = String::from_utf8(bytes)
            .map_err(|_| FiscalError::Artifact(format!("schema `{file_name}` is not UTF-8")))?;
        if text.starts_with('\u{feff}') {
            text.remove(0);
        }
        apply_pinned_regex_compatibility(&mut text, expected_hash)?;
        documents.insert((*file_name).to_string(), text);
    }
    Ok(documents)
}

fn apply_pinned_regex_compatibility(
    text: &mut String,
    expected_hash: &str,
) -> Result<(), FiscalError> {
    if expected_hash != PRODUCTION_SIMPLE_TYPES_SHA256 {
        return Ok(());
    }
    if text.matches(OFFICIAL_SERIES_PATTERN).count() != 1 {
        return Err(FiscalError::Artifact(
            "pinned production series pattern changed unexpectedly".to_string(),
        ));
    }
    *text = text.replacen(OFFICIAL_SERIES_PATTERN, XSD_COMPATIBLE_SERIES_PATTERN, 1);
    Ok(())
}

fn schema_assembly_error(error: oxixml_schema::SchemaError) -> FiscalError {
    FiscalError::Artifact(format!(
        "official schema assembly failed [{}]: {}",
        bounded(error.code(), 96),
        bounded(error.message(), 512)
    ))
}

fn bounded(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_manifest_fails_before_touching_the_filesystem() {
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
        assert!(
            NfseDpsSchemaValidator::from_pinned_directory("/does/not/matter", &UNKNOWN).is_err()
        );
    }

    #[test]
    fn compatibility_rewrite_is_hash_and_occurrence_bound() {
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
    }
}
