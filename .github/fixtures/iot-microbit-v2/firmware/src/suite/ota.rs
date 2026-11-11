//! Signed OTA verification: accepted, rejected and timed.

use super::{Ctx, FIRMWARE, TARGET, manager, manifest};
use crate::{Ms, expected as x, outln, store::RamCounterStore, timed};
use rullst_iot::{BootPartition, OtaError, OtaManager, OtaManifest, OtaStatus};

/// RAM copy for the tampering cases (kept out of the heap measurement).
static mut TAMPER_BUF: [u8; 32 * 1024] = [0; 32 * 1024];

fn tampered_firmware(index: usize) -> &'static [u8] {
    // SAFETY: single-threaded firmware; the buffer is only used from here.
    let buf = unsafe { &mut *core::ptr::addr_of_mut!(TAMPER_BUF) };
    buf.copy_from_slice(FIRMWARE);
    buf[index] ^= 0x01;
    buf
}

/// Verifies with a fresh manager (store at 41) and expects `want`, plus a
/// clean failed state: no pending manifest, commit refused, store untouched.
#[allow(clippy::too_many_arguments)]
fn reject(
    ctx: &mut Ctx,
    name: &str,
    public_key: &[u8],
    manifest: Result<OtaManifest, OtaError>,
    firmware: &[u8],
    signature: &[u8],
    want: OtaError,
) {
    let mut store = RamCounterStore::new(41);
    match (
        manager(&mut store, public_key, BootPartition::PartitionA),
        manifest,
    ) {
        (Ok(mut ota), Ok(m)) => {
            let (r, c) = timed(|| ota.verify_update(&m, firmware, signature));
            let clean = ota.status() == OtaStatus::Failed
                && ota.pending_manifest().is_none()
                && ota.commit_verified_update_with_store(&mut store)
                    == Err(OtaError::NoVerifiedUpdate)
                && store.value == 41;
            match r {
                Err(e) if e == want && clean => {
                    ctx.check(name, true, format_args!("rejected {:?} in {}", e, Ms(c)))
                }
                other => ctx.check(
                    name,
                    false,
                    format_args!("got {:?} want {:?} clean={}", other, want, clean),
                ),
            }
        }
        (m, n) => ctx.check(
            name,
            false,
            format_args!("setup {:?} {:?}", m.err(), n.err()),
        ),
    }
}

pub(super) fn ota(ctx: &mut Ctx) {
    // Manifest encoding and SHA-256 on device versus host.
    let (m42, hash_cycles) = timed(|| manifest(TARGET, "1.1.0", 42, FIRMWARE));
    let Ok(m42) = m42 else {
        ctx.check(
            "ota_manifest_build",
            false,
            format_args!("from_firmware failed"),
        );
        return;
    };
    ctx.bytes(
        "ota_manifest_sha256",
        &m42.firmware_sha256(),
        x::OTA_FIRMWARE_SHA256,
    );
    match m42.signing_bytes() {
        Ok(bytes) => ctx.bytes("ota_signing_bytes", &bytes, x::OTA_SIGNING_BYTES_42),
        Err(e) => ctx.check("ota_signing_bytes", false, format_args!("{:?}", e)),
    }
    let kib_per_s =
        (FIRMWARE.len() as u64 * u64::from(crate::CPU_HZ)) / (u64::from(hash_cycles.max(1)) * 1024);
    outln!(
        "INFO time sha256_32KiB(from_firmware) {} ({} cycles, ~{} KiB/s)",
        Ms(hash_cycles),
        hash_cycles,
        kib_per_s
    );

    // Valid update: accepted, then committed through the store.
    let mut store = RamCounterStore::new(41);
    match manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA) {
        Ok(mut ota) => {
            let (result, cycles) = timed(|| ota.verify_update(&m42, FIRMWARE, x::OTA_SIG_42));
            ctx.check(
                "ota_valid_accepted",
                result.is_ok() && ota.status() == OtaStatus::Verified,
                format_args!(
                    "{:?} status={:?} verify_32KiB={} ({} cycles)",
                    result,
                    ota.status(),
                    Ms(cycles),
                    cycles
                ),
            );
            let target = ota.verified_target_partition();
            ctx.check(
                "ota_target_is_inactive_bank",
                target == Ok(BootPartition::PartitionB),
                format_args!("running=A target={:?}", target),
            );
            let receipt = ota.commit_verified_update_with_store(&mut store);
            let ok = matches!(&receipt, Ok(r) if r.rollback_counter() == 42
                && r.version() == "1.1.0" && r.target_partition() == BootPartition::PartitionB)
                && store.value == 42
                && ota.firmware_version() == "1.1.0"
                && ota.rollback_counter() == 42
                && ota.status() == OtaStatus::Idle
                && ota.current_partition() == BootPartition::PartitionA
                && ota.pending_manifest().is_none();
            ctx.check(
                "ota_commit_with_store",
                ok,
                format_args!(
                    "store={} version={} status={:?} current={:?}",
                    store.value,
                    ota.firmware_version(),
                    ota.status(),
                    ota.current_partition()
                ),
            );
        }
        Err(e) => ctx.check(
            "ota_valid_accepted",
            false,
            format_args!("manager: {:?}", e),
        ),
    }

    // Ed25519-dominated timing: 16-byte firmware, three iterations.
    if let (Ok(tiny), Ok(mut ota)) = (
        manifest(TARGET, "1.0.1", 7, x::OTA_TINY_FIRMWARE),
        manager(
            &mut RamCounterStore::new(0),
            x::OTA_PUBLIC_KEY,
            BootPartition::PartitionA,
        ),
    ) {
        let mut all_ok = true;
        let (mut min, mut max) = (u32::MAX, 0u32);
        for _ in 0..3 {
            let (r, c) =
                timed(|| ota.verify_update(&tiny, x::OTA_TINY_FIRMWARE, x::OTA_SIG_TINY_7));
            all_ok &= r.is_ok();
            min = min.min(c);
            max = max.max(c);
        }
        ctx.check(
            "ota_verify_tiny_x3",
            all_ok,
            format_args!(
                "ed25519 verify_strict+sha256(16B) min={} max={} ({}..{} cycles)",
                Ms(min),
                Ms(max),
                min,
                max
            ),
        );
    } else {
        ctx.check("ota_verify_tiny_x3", false, format_args!("setup failed"));
    }

    // Rejections. Each case uses a fresh manager whose store holds 41.
    let tampered = tampered_firmware(12_345);
    reject(
        ctx,
        "ota_reject_tampered_firmware_byte",
        x::OTA_PUBLIC_KEY,
        manifest(TARGET, "1.1.0", 42, tampered),
        tampered,
        x::OTA_SIG_42,
        OtaError::SignatureInvalid,
    );
    reject(
        ctx,
        "ota_reject_firmware_vs_signed_hash",
        x::OTA_PUBLIC_KEY,
        manifest(TARGET, "1.1.0", 42, FIRMWARE),
        tampered,
        x::OTA_SIG_42,
        OtaError::FirmwareHashMismatch,
    );
    let mut bad_sig = [0u8; 64];
    bad_sig.copy_from_slice(x::OTA_SIG_42);
    bad_sig[10] ^= 0x80;
    reject(
        ctx,
        "ota_reject_tampered_signature",
        x::OTA_PUBLIC_KEY,
        manifest(TARGET, "1.1.0", 42, FIRMWARE),
        FIRMWARE,
        &bad_sig,
        OtaError::SignatureInvalid,
    );
    reject(
        ctx,
        "ota_reject_tampered_version",
        x::OTA_PUBLIC_KEY,
        manifest(TARGET, "1.1.1", 42, FIRMWARE),
        FIRMWARE,
        x::OTA_SIG_42,
        OtaError::SignatureInvalid,
    );
    reject(
        ctx,
        "ota_reject_truncated_firmware",
        x::OTA_PUBLIC_KEY,
        manifest(TARGET, "1.1.0", 42, FIRMWARE),
        &FIRMWARE[..FIRMWARE.len() - 1],
        x::OTA_SIG_42,
        OtaError::FirmwareLengthMismatch {
            expected: 32_768,
            actual: 32_767,
        },
    );
    reject(
        ctx,
        "ota_reject_signature_63B",
        x::OTA_PUBLIC_KEY,
        manifest(TARGET, "1.1.0", 42, FIRMWARE),
        FIRMWARE,
        &x::OTA_SIG_42[..63],
        OtaError::InvalidSignatureEncoding,
    );
    reject(
        ctx,
        "ota_reject_wrong_key",
        x::OTA_WRONG_PUBLIC_KEY,
        manifest(TARGET, "1.1.0", 42, FIRMWARE),
        FIRMWARE,
        x::OTA_SIG_42,
        OtaError::SignatureInvalid,
    );
    reject(
        ctx,
        "ota_reject_wrong_target",
        x::OTA_PUBLIC_KEY,
        manifest("nrf52840-dk", "1.1.0", 42, FIRMWARE),
        FIRMWARE,
        x::OTA_SIG_WRONG_TARGET,
        OtaError::TargetMismatch,
    );

    // Rollback: store already past (50) or equal to (42) the signed counter.
    for (name, stored) in [
        ("ota_reject_rollback_lower", 50u64),
        ("ota_reject_rollback_equal", 42),
    ] {
        let mut store = RamCounterStore::new(stored);
        let r = manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA)
            .and_then(|mut ota| ota.verify_update(&m42, FIRMWARE, x::OTA_SIG_42));
        ctx.expect_err(
            name,
            r,
            OtaError::RollbackDetected {
                current: stored,
                proposed: 42,
            },
        );
    }

    // Partition B, key validation, commit-without-verify, legacy APIs.
    let mut store = RamCounterStore::new(41);
    let r =
        manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionB).and_then(|mut ota| {
            ota.verify_update(&m42, FIRMWARE, x::OTA_SIG_42)?;
            ota.verified_target_partition()
        });
    ctx.check(
        "ota_running_b_targets_a",
        r == Ok(BootPartition::PartitionA),
        format_args!("{:?}", r),
    );
    ctx.expect_err(
        "ota_reject_weak_key",
        manager(
            &mut RamCounterStore::new(0),
            &[0u8; 32],
            BootPartition::PartitionA,
        ),
        OtaError::InvalidTrustedKey,
    );
    let r = manager(
        &mut RamCounterStore::new(0),
        x::OTA_PUBLIC_KEY,
        BootPartition::PartitionA,
    )
    .and_then(|mut ota| ota.commit_verified_update());
    ctx.expect_err("ota_commit_without_verify", r, OtaError::NoVerifiedUpdate);
    #[allow(deprecated)]
    let legacy = OtaManager::new("1.0.0").err();
    ctx.check(
        "ota_legacy_api_fails_closed",
        matches!(legacy, Some(OtaError::LegacyApiUnsupported { .. })),
        format_args!("{:?}", legacy),
    );
}
