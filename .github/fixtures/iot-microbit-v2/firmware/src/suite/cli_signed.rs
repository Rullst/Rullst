//! An update signed on the host by `cargo rullst iot:keygen` and `iot:sign`
//! (built in only with `RULLST_IOT_SIGNED_DIR`, see `build.rs`): the device
//! accepts it and rejects a tampered image, a tampered manifest and another key.

use super::{Ctx, TARGET, manager};
use crate::{Ms, expected as x, outln, store::RamCounterStore, timed};
use rullst_iot::{BootPartition, OtaError, OtaManifest};

static IMAGE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cli-image.bin"));
static MANIFEST: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cli.manifest"));
static SIGNATURE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cli.sig"));
static PUBLIC_KEY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cli.pub"));
const CHUNK: usize = 4 * 1024;
/// One chunk or the manifest in RAM, for the tampered cases.
static mut BUF: [u8; CHUNK] = [0; CHUNK];

fn buffer() -> &'static mut [u8; CHUNK] {
    // SAFETY: single-threaded firmware; each case finishes with the buffer
    // before the next one takes it.
    unsafe { &mut *core::ptr::addr_of_mut!(BUF) }
}

/// Verifies `manifest` and the image (byte `flip` changed, if any) with a
/// manager that trusts `key` and committed `counter - 1`.
fn verify(
    manifest: &OtaManifest,
    signature: &[u8],
    key: &[u8],
    flip: Option<usize>,
) -> Result<(), OtaError> {
    let mut store = RamCounterStore::new(manifest.rollback_counter().saturating_sub(1));
    let mut ota = manager(&mut store, key, BootPartition::PartitionA)?;
    let mut verifier = ota.begin_update(manifest, signature)?;
    for (index, chunk) in IMAGE.chunks(CHUNK).enumerate() {
        match flip {
            Some(at) if at / CHUNK == index => {
                let buf = buffer();
                buf[..chunk.len()].copy_from_slice(chunk);
                buf[at % CHUNK] ^= 0x01;
                verifier.update(&buf[..chunk.len()])?;
            }
            _ => verifier.update(chunk)?,
        }
    }
    verifier.finish()
}

pub(super) fn cli_signed(ctx: &mut Ctx) {
    let parsed = OtaManifest::from_signing_bytes(MANIFEST);
    let Ok(manifest) = parsed else {
        ctx.check(
            "cli_manifest_parsed",
            false,
            format_args!("{:?}", parsed.err()),
        );
        return;
    };
    ctx.check(
        "cli_manifest_parsed",
        manifest.target() == TARGET && manifest.firmware_len() == IMAGE.len() as u64,
        format_args!(
            "{} bytes: target={} version={} counter={} image={}B",
            MANIFEST.len(),
            manifest.target(),
            manifest.version(),
            manifest.rollback_counter(),
            manifest.firmware_len()
        ),
    );
    let (accepted, cycles) = timed(|| verify(&manifest, SIGNATURE, PUBLIC_KEY, None));
    ctx.check(
        "cli_signed_update_accepted",
        accepted.is_ok(),
        format_args!(
            "{:?} in {} ({}B in 4 KiB chunks)",
            accepted,
            Ms(cycles),
            IMAGE.len()
        ),
    );
    let middle = IMAGE.len() / 2;
    ctx.expect_err(
        "cli_signed_tampered_image_rejected",
        verify(&manifest, SIGNATURE, PUBLIC_KEY, Some(middle)),
        OtaError::FirmwareHashMismatch,
    );
    // The version text's last byte: still a valid manifest, no longer signed.
    let buf = buffer();
    buf[..MANIFEST.len()].copy_from_slice(MANIFEST);
    let version_end = 23 + 4 + manifest.target().len() + 4 + manifest.version().len() - 1;
    buf[version_end] ^= 0x01;
    let tampered = OtaManifest::from_signing_bytes(&buf[..MANIFEST.len()]);
    match tampered {
        Ok(tampered) => ctx.expect_err(
            "cli_signed_tampered_manifest_rejected",
            verify(&tampered, SIGNATURE, PUBLIC_KEY, None),
            OtaError::SignatureInvalid,
        ),
        Err(e) => ctx.check(
            "cli_signed_tampered_manifest_rejected",
            false,
            format_args!("parse {:?}", e),
        ),
    }
    ctx.expect_err(
        "cli_signed_other_key_rejected",
        verify(&manifest, SIGNATURE, x::OTA_PUBLIC_KEY, None),
        OtaError::SignatureInvalid,
    );
    outln!(
        "INFO cli_signed manifest={}B signature={}B",
        MANIFEST.len(),
        SIGNATURE.len()
    );
}
