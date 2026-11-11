//! The rollback-counter compare-and-set contract against an in-RAM store.

use super::{Ctx, FIRMWARE, TARGET, manager, manifest};
use crate::{expected as x, store::RamCounterStore};
use rullst_iot::{BootPartition, OtaError, OtaStatus, RollbackCounterError, RollbackCounterStore};

pub(super) fn cas(ctx: &mut Ctx) {
    // Raw contract on the in-RAM store.
    let mut store = RamCounterStore::new(41);
    let ok = store.compare_and_set(41, 42).is_ok() && store.load() == Ok(42);
    ctx.check(
        "cas_success",
        ok,
        format_args!("41->42 stored={}", store.value),
    );
    let r = store.compare_and_set(41, 43);
    ctx.check(
        "cas_stale_expected",
        r == Err(RollbackCounterError::Conflict {
            expected: 41,
            actual: 42,
        }) && store.value == 42,
        format_args!("{:?} stored={}", r, store.value),
    );
    let r = store.compare_and_set(42, 42);
    ctx.check(
        "cas_non_monotonic",
        r == Err(RollbackCounterError::NonMonotonic {
            current: 42,
            proposed: 42,
        }) && store.value == 42,
        format_args!("{:?}", r),
    );

    // Concurrent-style double use: two managers loaded from the same store.
    let (Ok(m43), Ok(m44)) = (
        manifest(TARGET, "1.1.0", 43, FIRMWARE),
        manifest(TARGET, "1.1.0", 44, FIRMWARE),
    ) else {
        ctx.check("cas_setup", false, format_args!("manifest build failed"));
        return;
    };
    let mut store = RamCounterStore::new(42);
    let first = manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA);
    let second = manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA);
    let third = manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA);
    let (Ok(mut first), Ok(mut second), Ok(mut third)) = (first, second, third) else {
        ctx.check("cas_setup", false, format_args!("manager build failed"));
        return;
    };
    let verified = first.verify_update(&m43, FIRMWARE, x::OTA_SIG_43).is_ok()
        && second.verify_update(&m44, FIRMWARE, x::OTA_SIG_44).is_ok()
        && third.verify_update(&m43, FIRMWARE, x::OTA_SIG_43).is_ok();
    let first_commit = first.commit_verified_update_with_store(&mut store);
    ctx.check(
        "cas_double_use_first_wins",
        verified
            && matches!(&first_commit, Ok(r) if r.rollback_counter() == 43)
            && store.value == 43,
        format_args!(
            "{:?} stored={}",
            first_commit.as_ref().map(|r| r.rollback_counter()),
            store.value
        ),
    );
    let second_commit = second.commit_verified_update_with_store(&mut store);
    ctx.check(
        "cas_double_use_stale_newer_refused",
        second_commit
            == Err(OtaError::RollbackCounterStore(
                RollbackCounterError::Conflict {
                    expected: 42,
                    actual: 43,
                },
            ))
            && store.value == 43
            && second.status() == OtaStatus::Verified,
        format_args!(
            "{:?} stored={} status={:?}",
            second_commit,
            store.value,
            second.status()
        ),
    );
    let third_commit = third.commit_verified_update_with_store(&mut store);
    ctx.check(
        "cas_double_use_same_counter_refused",
        third_commit
            == Err(OtaError::RollbackCounterStore(
                RollbackCounterError::Conflict {
                    expected: 42,
                    actual: 43,
                },
            ))
            && store.value == 43
            && store.writes == 1,
        format_args!("{:?} writes={}", third_commit, store.writes),
    );
    // A fresh manager (as after a reboot) refuses the committed manifest.
    let replay = manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA)
        .and_then(|mut ota| ota.verify_update(&m43, FIRMWARE, x::OTA_SIG_43));
    ctx.expect_err(
        "cas_replay_after_restart",
        replay,
        OtaError::RollbackDetected {
            current: 43,
            proposed: 43,
        },
    );

    // Outcome-unknown write (acknowledgement lost) is completed on retry.
    let mut store = RamCounterStore::new(43);
    store.lose_next_ack = Some(RollbackCounterError::Unavailable);
    let result =
        manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA).and_then(|mut ota| {
            ota.verify_update(&m44, FIRMWARE, x::OTA_SIG_44)?;
            let first = ota.commit_verified_update_with_store(&mut store);
            let retry = ota.commit_verified_update_with_store(&mut store)?;
            Ok((first, retry.rollback_counter(), ota.rollback_counter()))
        });
    ctx.check(
        "cas_lost_ack_retry_completes",
        matches!(
            result,
            Ok((
                Err(OtaError::RollbackCounterStore(
                    RollbackCounterError::Unavailable
                )),
                44,
                44
            ))
        ) && store.value == 44
            && store.writes == 1,
        format_args!(
            "{:?} stored={} writes={}",
            result, store.value, store.writes
        ),
    );
    let mut store = RamCounterStore::new(1);
    store.fail_next_load = Some(RollbackCounterError::Unavailable);
    ctx.expect_err(
        "cas_load_unavailable",
        manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA),
        OtaError::RollbackCounterStore(RollbackCounterError::Unavailable),
    );
}
