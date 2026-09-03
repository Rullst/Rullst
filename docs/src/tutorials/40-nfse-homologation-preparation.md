# Preparing a National NFS-e 1.01 homologation candidate

> [!IMPORTANT]
> Dependency examples use `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use path dependencies from
> this source checkout during development.

This guide exercises the part of the Brazilian National NFS-e pipeline that can
be proved safely without sending a fiscal document. It builds a bounded DPS,
checks checksum-pinned government schema sources, signs the document with an
application-supplied A1 PKCS#12 certificate, and validates the signed result
again.

It does **not** authorize a note. Rullst keeps homologation and production
transmission disabled until the remaining protocol and external-evidence gates
listed below are complete.

Enable the isolated dependency boundary before following the signing steps:

```toml
[dependencies]
rullst-capital = { version = "12.0.0-rc.1", features = ["nfse"] }
```

Umbrella applications can select `rullst = { version = "12.0.0-rc.1", features =
["capital-nfse"] }`. The feature adds local schema/signature/codec dependencies;
it does not enable a live network path.

## 1. Know the pinned contracts

This source revision recognizes only these immutable artifact profiles:

| Environment | Official archive | Archive SHA-256 |
| :--- | :--- | :--- |
| Production | `NFSe-ESQUEMAS_XSD-v1.01-20260209.zip` | `e7935cbd9470527c6cc32984c1b2263e614183bf0139ce2733eaaed2de9a8072` |
| Restricted production | `NFSe-ESQUEMAS_XSD-PRODREST-v1.01-20260727.zip` | `6c7e0510d3ecff4454f291f4e10b742d27a4818f23aab181494f96d0ea79f3dc` |

Download production artifacts from the official [current technical
documentation](https://www.gov.br/nfse/pt-br/biblioteca/documentacao-tecnica/documentacao-atual)
page and restricted artifacts from the official [restricted-production
documentation](https://www.gov.br/nfse/pt-br/biblioteca/documentacao-tecnica/producao-restrita)
page. Do not copy an archive from an unofficial mirror.

Verify the archive before extraction:

```bash
sha256sum NFSe-ESQUEMAS_XSD-v1.01-20260209.zip
```

`NfseDpsSchemaValidator` also verifies every expected XSD file. It rejects an
unknown profile, a missing/modified file, a file larger than 256 KiB, traversal,
and any import outside its closed in-memory catalogue. The XML instance cannot
make it download a schema or follow a filesystem hint.

There is one intentionally visible compatibility rule. The pinned production
simple-types file contains the DPS-series pattern `^0{0,4}\d{1,5}$`, authored
with `.NET` anchors even though `^` and `$` are literals in XSD regex grammar.
Only after the file hash matches, Rullst removes those two anchors in memory.
The rewrite is tied to that exact file hash and exact one occurrence; any
upstream change fails closed. Restricted production currently needs no rewrite.

## 2. Exercise the unsigned builder and official XSD

Point the example at the directory that directly contains `DPS_v1.01.xsd`:

```bash
RULLST_NFSE_XSD_DIR=/path/to/extracted/Schemas/1.01 \
  cargo run -p rullst-capital --example nfse_v101_preview
```

The example uses `NfseDpsV101`, not the legacy floating-point preview. Its
bounded subset is an ordinary domestic service with:

- integer BRL cents and ISS basis points;
- explicit taxation and retention enums;
- checked CPF/CNPJ digits, IBGE codes, DPS ID, series, service code, text, and
  XML size;
- no automatic guess about municipal parameters or tax treatment.

Passing XSD validation proves document structure and scalar constraints only.
SEFIN business rules still depend on the emitter, municipality, contributor
registration, Simples Nacional state, service classification, and current
official parameters.

## 3. Load and sign with an A1 certificate

Never commit a `.pfx`/`.p12` file or its passphrase. Load them from the
deployment secret boundary and keep the passphrase out of logs:

```rust,no_run
use rullst_capital::fiscal::{
    FiscalCertificate, NFSE_RESTRICTED_V1_01_20260727,
    NfseDpsSchemaValidator, sign_dps_xml,
};

fn prepare_signed_candidate(
    unsigned_xml: &str,
    schema_directory: &std::path::Path,
    pkcs12_path: &std::path::Path,
    passphrase: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let pkcs12 = std::fs::read(pkcs12_path)?;
    let certificate = FiscalCertificate::from_bytes(&pkcs12, passphrase)?;
    let validator = NfseDpsSchemaValidator::from_pinned_directory(
        schema_directory,
        &NFSE_RESTRICTED_V1_01_20260727,
    )?;

    validator.validate(unsigned_xml)?;
    let signed_xml = sign_dps_xml(unsigned_xml, &certificate)?;
    validator.validate(&signed_xml)?;
    Ok(signed_xml)
}
```

`sign_dps_xml` accepts exactly one unsigned DPS 1.01 envelope with a unique
45-character `infDPS/@Id`. It extracts the matching key/certificate chain from
PKCS#12, requires an RSA PKCS#8 key, emits an enveloped RSA-SHA256 signature
using SHA-256 and inclusive C14N 1.0, and refuses to return partially signed
XML. The signing path verifies the result against its embedded certificate
before returning it; separate tests also verify the generated XMLDSig and
validate a signed builder fixture with the pinned official schema.

Rullst redacts the certificate container in `Debug` and zeroizes certificate
bytes, passphrases, decoded base64, and derived PEM buffers it owns. The
application still owns secret-file permissions, secret-manager integration,
rotation, process/core-dump policy, and access auditing.

## 4. Run the opt-in official-artifact regression

The repository does not redistribute mutable government packages. After
downloading and verifying the pinned production archive, run the ignored test:

```bash
RULLST_NFSE_XSD_DIR=/path/to/extracted/Schemas/1.01 \
  cargo test -p rullst-capital \
  fiscal::signer::tests::signed_builder_output_matches_the_official_xsd_when_supplied \
  -- --ignored
```

The test generates an ephemeral RSA key and certificate at runtime. It is a
schema/cryptographic interoperability fixture, not an ICP-Brasil certificate or
an official homologation result.

The corresponding restricted-production package has a separate immutable
manifest and regression:

```bash
RULLST_NFSE_RESTRICTED_XSD_DIR=/path/to/extracted/restricted/schemas \
  cargo test -p rullst-capital \
  fiscal::signer::tests::signed_builder_output_matches_the_official_restricted_xsd_when_supplied \
  -- --ignored
```

## 5. Exercise the offline protocol boundary

After producing the signed DPS, build the exact request body without sending it:

```rust,no_run
use rullst_capital::fiscal::NfseIssueRequest;

# fn prepare(signed_dps: &str) -> Result<Vec<u8>, rullst_capital::fiscal::FiscalError> {
let request = NfseIssueRequest::try_from_signed_dps(signed_dps)?;
let body = request.to_json()?;
# Ok(body)
# }
```

Construction verifies that the document has one unique official DPS ID, one
direct signed `infDPS/tpAmb`, one direct XMLDSig reference to that ID and a
cryptographically valid embedded signature. GZip output fixes its timestamp to
zero, so the JSON is deterministic for a given signed document. The environment
passed to `parse_response` must equal that signed `tpAmb`.

For retained protocol fixtures, call `request.parse_response(status,
environment, body)`. HTTP 201 is represented only as
`NfseIssueResponse::Authorized`; HTTP 400, 403 and 500 are represented as
`Rejected`. The parser rejects unknown fields, wrong environments or DPS IDs,
invalid access keys, malformed JSON/Base64/GZip/XML, unsigned or tampered NFS-e
XML and decompressed material above four MiB. Embedded-signature verification
proves document integrity against its declared certificate, not ICP-Brasil trust
or emitter ownership.

## 6. Journal a caller-owned command before transport

The `nfse` feature provides bounded local evidence for a future reviewed
transport workflow. Load exactly 32 random bytes from a secret manager; never
derive this key directly from a password or commit it to the repository.

```rust,no_run
use rullst_capital::fiscal::{
    FiscalCommandJournal, FiscalJournalDisposition, FiscalJournalKey,
    NfseEnvironment, NfseIssueRequest, NfseIssueResponse,
};
use std::path::Path;

# fn stage(
#     path: &Path,
#     key_bytes: &[u8],
#     request: &NfseIssueRequest,
#     parsed_response: Option<&NfseIssueResponse>,
# ) -> Result<(), Box<dyn std::error::Error>> {
let key = FiscalJournalKey::try_new("fiscal-2026-01", key_bytes)?;
let journal = FiscalCommandJournal::try_open(path, key)?;

// Use an opaque, stable, non-PII ID from the application's authoritative outbox.
let prepared = journal.prepare(
    "nfse-command:01J8YQ2V5M",
    NfseEnvironment::Homologation,
    request,
)?;
if prepared.disposition() == FiscalJournalDisposition::Recorded {
    // A future reviewed transport may run only after this synchronized record.
}

if let Some(response) = parsed_response {
    journal.record_response("nfse-command:01J8YQ2V5M", request, response)?;
}

for unresolved in journal.pending()? {
    // Locate the real request in the application's protected outbox, compare
    // request_digest(), and reconcile with SEFIN before deciding whether to retry.
    let _command_id = unresolved.command_id();
}

// Persist this separately if valid-prefix truncation must be detected on restart.
let _checkpoint = journal.checkpoint()?;
# Ok(())
# }
```

The HMAC-chained file accepts one active writer and at most 4,096 events/16 MiB.
It stores no DPS/NFS-e XML, access key, processing message, response body, or
certificate material. An exact command/request/result replay does not append;
conflicts, corrupted/reordered frames, wrong keys, symlinks, external growth,
quota exhaustion, and uncertain synchronization fail closed. `pending()` is a
recovery index, not the protected request store itself. The host owns key
rotation, directory permissions, exclusive-writer enforcement, the real
request/outbox, independent checkpoint retention, backup/retention, retry
policy, and authority reconciliation. The journal does not perform network I/O
or establish exactly-once behavior across systems.

## 7. Do not enable live transmission yet

The official endpoints are immutable in `NfseEnvironment`, and the live path
can already build an HTTPS-only rustls mTLS client with redirects disabled and
bounded connect/request timeouts. It deliberately performs no request.

The exact request envelope and bounded response/rejection codec are now local,
fixture-testable prerequisites. Before Rullst can enable restricted-production
transmission, the exact source revision must still have:

1. retained fixtures from the current official restricted-production contract,
   including every supported success/rejection shape;
2. certificate validity, key-usage, emitter CPF/CNPJ, and ICP-Brasil chain
   policy reviewed against the current national rules;
3. deployment of the local journal with protected authoritative request/outbox
   storage, exclusive-writer/key/checkpoint operations, retry policy, and
   explicit authority reconciliation;
4. positive and negative tests in the restricted environment using an
   authorized real A1 certificate and valid contributor/municipality data;
5. independent fiscal/security review and retained evidence tied to an
   immutable commit;
6. successful official homologation before production can be considered.

Until those gates pass, `NfseEnvironment::Homologation` and
`NfseEnvironment::Production` return `FiscalError::Unsupported`, while
`NfseEnvironment::Mock` remains unmistakably `MOCK_NOT_AUTHORIZED`.
