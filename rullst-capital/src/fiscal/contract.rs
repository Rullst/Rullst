//! Versioned, immutable boundaries for the Brazilian National NFS-e contract.

/// Official NFS-e XML namespace shared by DPS and NFS-e documents.
pub const NFSE_NAMESPACE: &str = "http://www.sped.fazenda.gov.br/nfse";

/// Official XMLDSig namespace imported by the NFS-e schemas.
pub const XMLDSIG_NAMESPACE: &str = "http://www.w3.org/2000/09/xmldsig#";

/// Maximum DPS request accepted by the Rullst fiscal boundary.
pub const MAX_DPS_XML_BYTES: usize = 1024 * 1024;

/// Maximum SEFIN response accepted before parsing.
pub const MAX_SEFIN_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// An immutable set of official artifacts against which an integration was built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NfseArtifactManifest {
    /// Stable Rullst profile identifier.
    pub profile: &'static str,
    /// DPS/NFS-e schema version.
    pub schema_version: &'static str,
    /// Date carried by the official artifact package.
    pub artifact_date: &'static str,
    /// Official schema archive file name.
    pub schema_archive: &'static str,
    /// SHA-256 measured from the downloaded official archive.
    pub schema_archive_sha256: &'static str,
    /// Root schema for an unsigned or signed DPS.
    pub dps_root_schema: &'static str,
    /// Root schema for an authorized NFS-e response.
    pub nfse_root_schema: &'static str,
    /// Government page that publishes the artifact.
    pub documentation_url: &'static str,
}

/// Current production contract pinned by this Rullst source revision.
///
/// Updating these values is a reviewed compatibility change. Runtime code never downloads a
/// schema or follows an XML-provided schema location.
pub const NFSE_PRODUCTION_V1_01_20260209: NfseArtifactManifest = NfseArtifactManifest {
    profile: "nfse-production-v1.01-20260209",
    schema_version: "1.01",
    artifact_date: "2026-02-09",
    schema_archive: "NFSe-ESQUEMAS_XSD-v1.01-20260209.zip",
    schema_archive_sha256: "e7935cbd9470527c6cc32984c1b2263e614183bf0139ce2733eaaed2de9a8072",
    dps_root_schema: "DPS_v1.01.xsd",
    nfse_root_schema: "NFSe_v1.01.xsd",
    documentation_url: "https://www.gov.br/nfse/pt-br/biblioteca/documentacao-tecnica/documentacao-atual",
};

/// Current restricted-production contract pinned for future official homologation tests.
pub const NFSE_RESTRICTED_V1_01_20260727: NfseArtifactManifest = NfseArtifactManifest {
    profile: "nfse-restricted-v1.01-20260727",
    schema_version: "1.01",
    artifact_date: "2026-07-27",
    schema_archive: "NFSe-ESQUEMAS_XSD-PRODREST-v1.01-20260727.zip",
    schema_archive_sha256: "6c7e0510d3ecff4454f291f4e10b742d27a4818f23aab181494f96d0ea79f3dc",
    dps_root_schema: "DPS_v1.01.xsd",
    nfse_root_schema: "NFSe_v1.01.xsd",
    documentation_url: "https://www.gov.br/nfse/pt-br/biblioteca/documentacao-tecnica/producao-restrita",
};

/// Official SEFIN service boundaries. They cannot be overridden with an arbitrary URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NfseSefinContract {
    /// HTTPS origin and API prefix.
    pub base_url: &'static str,
    /// Synchronous DPS issuance path.
    pub issue_path: &'static str,
    /// Manifest against which XML must be validated.
    pub artifacts: &'static NfseArtifactManifest,
}

/// Official restricted-production SEFIN contract.
pub const NFSE_RESTRICTED_SEFIN: NfseSefinContract = NfseSefinContract {
    base_url: "https://sefin.producaorestrita.nfse.gov.br/API/SefinNacional",
    issue_path: "/nfse",
    artifacts: &NFSE_RESTRICTED_V1_01_20260727,
};

/// Official production SEFIN contract.
pub const NFSE_PRODUCTION_SEFIN: NfseSefinContract = NfseSefinContract {
    base_url: "https://sefin.nfse.gov.br/SefinNacional",
    issue_path: "/nfse",
    artifacts: &NFSE_PRODUCTION_V1_01_20260209,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_contracts_are_https_and_not_runtime_configurable() {
        for contract in [NFSE_RESTRICTED_SEFIN, NFSE_PRODUCTION_SEFIN] {
            assert!(contract.base_url.starts_with("https://"));
            assert_eq!(contract.issue_path, "/nfse");
            assert_eq!(contract.artifacts.schema_version, "1.01");
            assert_eq!(contract.artifacts.schema_archive_sha256.len(), 64);
        }
    }
}
