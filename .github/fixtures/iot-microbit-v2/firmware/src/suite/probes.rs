//! Per-call stack depth, and the allocation-failure check run after the
//! memory report so that its 20 KiB payload does not count in the heap peak.

use super::{Ctx, FIRMWARE, TARGET, manager, manifest};
use crate::{HEAP, expected as x, outln, stack_depth, store::RamCounterStore};
use alloc::vec;
use rullst_iot::{
    AnomalyDetector, AnomalyState, BootPartition, CoapMessageType, CoapMethod, CoapRequest,
    MqttCodecError, MqttPublish, MqttQos, SensorTelemetry,
};

/// Stack depth of individual rullst-iot calls (fresh paint before each).
pub fn stack_probes() {
    let report = |name: &str, ok: bool, depth: usize| {
        outln!("INFO stack_depth {} {}B ok={}", name, depth, ok);
    };
    let telemetry = SensorTelemetry::new("microbit-v2-01", "temperature", 23.75, 1_762_790_400);
    let (r, d) = stack_depth(|| serde_json::to_vec(&telemetry).is_ok());
    report("serde_json::to_vec(SensorTelemetry)", r, d);
    let (r, d) =
        stack_depth(|| serde_json::from_slice::<SensorTelemetry>(x::TELEMETRY_JSON).is_ok());
    report("serde_json::from_slice(SensorTelemetry)", r, d);
    let (r, d) = stack_depth(|| {
        MqttPublish::reliable(
            "factory/line-1/temperature",
            b"24.5".to_vec(),
            MqttQos::AtLeastOnce,
            7,
        )
        .and_then(|p| p.encode())
        .is_ok()
    });
    report("MqttPublish::reliable+encode", r, d);
    let (r, d) = stack_depth(|| {
        CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Post,
            42,
            [0x01, 0x02],
        )
        .and_then(|r| r.path_segment("telemetry"))
        .map(|r| r.content_format(50))
        .and_then(|r| r.payload(br#"{"temperature":24.5}"#.to_vec()))
        .and_then(|r| r.encode())
        .is_ok()
    });
    report("CoapRequest::new+encode", r, d);
    let (r, d) =
        stack_depth(|| AnomalyDetector::new(25.0, 5.0).evaluate(31.0) == AnomalyState::Warning);
    report("AnomalyDetector::evaluate", r, d);
    let (m42, d) = stack_depth(|| manifest(TARGET, "1.1.0", 42, FIRMWARE));
    report("OtaManifest::from_firmware(32KiB)", m42.is_ok(), d);
    let mut store = RamCounterStore::new(41);
    let (ota, d) =
        stack_depth(|| manager(&mut store, x::OTA_PUBLIC_KEY, BootPartition::PartitionA));
    report("OtaManager::new_with_running_partition", ota.is_ok(), d);
    if let (Ok(m42), Ok(mut ota)) = (m42, ota) {
        let (r, d) = stack_depth(|| ota.verify_update(&m42, FIRMWARE, x::OTA_SIG_42).is_ok());
        report("OtaManager::verify_update(32KiB,valid)", r, d);
        let (r, d) = stack_depth(|| ota.commit_verified_update_with_store(&mut store).is_ok());
        report("OtaManager::commit_verified_update_with_store", r, d);
    }
}

/// A 20 KiB payload fits the 32 KiB heap; its encoded copy does not. Since
/// 13.0.0-alpha.5 `MqttPublish::encode` returns `AllocationFailed` here
/// instead of aborting the firmware.
pub fn allocation_failure_checks(ctx: &mut Ctx) {
    let publish = MqttPublish::new("probe/oom", vec![0x5Au8; 20 * 1024]);
    let live = HEAP.used();
    let result = publish
        .as_ref()
        .map_err(|e| *e)
        .and_then(|p| p.encode())
        .map(|bytes| bytes.len());
    let failures = HEAP.failures();
    ctx.check(
        "mqtt_encode_allocation_failure_is_an_error",
        result == Err(MqttCodecError::AllocationFailed) && failures > 0,
        format_args!(
            "{:?} heap_live={}B capacity={}B refused_allocations={}",
            result,
            live,
            crate::HEAP_SIZE,
            failures
        ),
    );
    drop(publish);
    let small = MqttPublish::new("probe/ok", vec![1u8; 16])
        .and_then(|p| p.encode())
        .map(|b| b.len());
    ctx.check(
        "mqtt_encode_after_allocation_failure",
        small == Ok(29),
        format_args!("{:?}", small),
    );
}
