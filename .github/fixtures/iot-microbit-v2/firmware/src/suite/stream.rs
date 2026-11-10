//! Streaming OTA verification: a manifest built from received fields and the
//! 32 KiB image verified in 4 KiB chunks, as if read from the network or from
//! external flash, so it is hashed once and never needed as one slice.

use super::{Ctx, FIRMWARE, TARGET, manager};
use crate::{Ms, expected as x, outln, store::RamCounterStore, timed};
use rullst_iot::{BootPartition, OtaError, OtaManifest, OtaStatus};

pub(super) const CHUNK: usize = 4 * 1024;
/// One chunk in RAM, for the tampered-chunk case.
static mut CHUNK_BUF: [u8; CHUNK] = [0; CHUNK];

fn signed_hash() -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash.copy_from_slice(x::OTA_FIRMWARE_SHA256);
    hash
}

/// The manifest as a device builds it from the fields it receives.
pub(super) fn received_manifest() -> Result<OtaManifest, OtaError> {
    OtaManifest::new(TARGET, "1.1.0", 42, FIRMWARE.len() as u64, signed_hash())
}

pub(super) fn stream(ctx: &mut Ctx) {
    let (from_fields, field_cycles) = timed(received_manifest);
    let published = OtaManifest::from_firmware(TARGET, "1.1.0", 42, FIRMWARE);
    ctx.check(
        "ota_manifest_from_fields",
        from_fields.is_ok() && from_fields == published,
        format_args!(
            "equals from_firmware without hashing, in {} ({} cycles)",
            Ms(field_cycles),
            field_cycles
        ),
    );
    let (parsed, parse_cycles) = timed(|| OtaManifest::from_signing_bytes(x::OTA_SIGNING_BYTES_42));
    let truncated = OtaManifest::from_signing_bytes(&x::OTA_SIGNING_BYTES_42[..94]);
    ctx.check(
        "ota_manifest_from_signing_bytes",
        parsed.is_ok() && parsed == published && truncated == Err(OtaError::MalformedManifest),
        format_args!(
            "95 host bytes parsed in {} ({} cycles); truncated -> {:?}",
            Ms(parse_cycles),
            parse_cycles,
            truncated.err()
        ),
    );
    let Ok(manifest) = from_fields else {
        return;
    };

    // Valid image in eight 4 KiB chunks, then committed through the store.
    let mut store = RamCounterStore::new(41);
    match manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA) {
        Ok(mut ota) => {
            // The verifier borrows the manager; `map` finishes it.
            let (begun, begin_cycles) = timed(|| ota.begin_update(&manifest, x::OTA_SIG_42));
            let outcome = begun.map(|mut verifier| {
                let mut chunk_ok = true;
                let (_, chunk_cycles) = timed(|| {
                    for chunk in FIRMWARE.chunks(CHUNK) {
                        chunk_ok &= verifier.update(chunk).is_ok();
                    }
                });
                let (finished, finish_cycles) = timed(|| verifier.finish());
                (chunk_ok, finished, chunk_cycles, finish_cycles)
            });
            match outcome {
                Ok((chunk_ok, finished, chunk_cycles, finish_cycles)) => {
                    let total = begin_cycles + chunk_cycles + finish_cycles;
                    ctx.check(
                        "ota_stream_4KiB_chunks",
                        chunk_ok && finished.is_ok() && ota.status() == OtaStatus::Verified,
                        format_args!(
                            "{:?} begin(signature)={} 8x4KiB={} ({}/chunk) finish={} total={} ({} cycles)",
                            finished,
                            Ms(begin_cycles),
                            Ms(chunk_cycles),
                            Ms(chunk_cycles / 8),
                            Ms(finish_cycles),
                            Ms(total),
                            total
                        ),
                    );
                    let target = ota.verified_target_partition();
                    let receipt = ota.commit_verified_update_with_store(&mut store);
                    ctx.check(
                        "ota_stream_commit_with_store",
                        target == Ok(BootPartition::PartitionB)
                            && matches!(&receipt, Ok(r) if r.rollback_counter() == 42)
                            && store.value == 42,
                        format_args!("target={:?} store={}", target, store.value),
                    );
                }
                Err(e) => ctx.check(
                    "ota_stream_4KiB_chunks",
                    false,
                    format_args!("begin: {:?}", e),
                ),
            }
        }
        Err(e) => ctx.check(
            "ota_stream_4KiB_chunks",
            false,
            format_args!("manager: {:?}", e),
        ),
    }

    // A bad signature is refused before the first chunk.
    let mut bad_sig = [0u8; 64];
    bad_sig.copy_from_slice(x::OTA_SIG_42);
    bad_sig[33] ^= 0x04;
    let mut store = RamCounterStore::new(41);
    let refused =
        manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA).map(|mut ota| {
            let (result, cycles) = timed(|| ota.begin_update(&manifest, &bad_sig).err());
            (result, cycles, ota.status())
        });
    ctx.check(
        "ota_stream_reject_signature_before_image",
        matches!(
            refused,
            Ok((Some(OtaError::SignatureInvalid), _, OtaStatus::Failed))
        ),
        format_args!(
            "{:?} in {}",
            refused.as_ref().map(|r| &r.0),
            Ms(refused.as_ref().map_or(0, |r| r.1))
        ),
    );

    // A flipped byte in chunk 3 fails the digest at `finish`.
    let mut store = RamCounterStore::new(41);
    let tampered =
        manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA).and_then(|mut ota| {
            let mut verifier = ota.begin_update(&manifest, x::OTA_SIG_42)?;
            for (index, chunk) in FIRMWARE.chunks(CHUNK).enumerate() {
                if index == 3 {
                    // SAFETY: single-threaded firmware; only this case uses the buffer.
                    let buf = unsafe { &mut *core::ptr::addr_of_mut!(CHUNK_BUF) };
                    buf.copy_from_slice(chunk);
                    buf[1234] ^= 0x80;
                    verifier.update(buf)?;
                } else {
                    verifier.update(chunk)?;
                }
            }
            verifier.finish()
        });
    ctx.expect_err(
        "ota_stream_reject_tampered_chunk",
        tampered,
        OtaError::FirmwareHashMismatch,
    );

    // A ninth chunk is refused before it is hashed; the verifier stays failed.
    let mut store = RamCounterStore::new(41);
    let oversize =
        manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA).and_then(|mut ota| {
            let mut verifier = ota.begin_update(&manifest, x::OTA_SIG_42)?;
            for chunk in FIRMWARE.chunks(CHUNK) {
                verifier.update(chunk)?;
            }
            let extra = verifier.update(&FIRMWARE[..CHUNK]);
            let received = verifier.received();
            let finish = verifier.finish();
            Ok((extra, received, finish))
        });
    let want = OtaError::FirmwareLengthMismatch {
        expected: 32_768,
        actual: 36_864,
    };
    ctx.check(
        "ota_stream_reject_oversize_chunk",
        matches!(&oversize, Ok((Err(a), 32_768, Err(b))) if *a == want && *b == want),
        format_args!("{:?}", oversize),
    );
    outln!(
        "INFO stream chunk={}B chunks={}",
        CHUNK,
        FIRMWARE.len() / CHUNK
    );
}
