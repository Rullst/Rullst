//! The encoders return a typed error, instead of aborting the process, when
//! the allocator refuses their buffer, as a small device heap does. This test
//! binary installs an allocator that fails, on the calling thread only, every
//! request at or above a limit.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ed25519_dalek::{Signer, SigningKey};
use rullst_iot::{
    CoapCodecError, CoapMessageType, CoapMethod, CoapRequest, MqttCodecError, MqttPublish, MqttQos,
    OtaError, OtaManager, OtaManifest, OtaStatus,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// Allocations of at least this many bytes fail on this thread.
    static LIMIT: Cell<usize> = const { Cell::new(usize::MAX) };
}

fn limit() -> usize {
    LIMIT.try_with(Cell::get).unwrap_or(usize::MAX)
}

struct LimitedAllocator;

// SAFETY: every call is forwarded unchanged to `System`, except requests at
// or above the limit, which return null as the `GlobalAlloc` contract allows.
unsafe impl GlobalAlloc for LimitedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() >= limit() {
            return std::ptr::null_mut();
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size >= limit() {
            return std::ptr::null_mut();
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: LimitedAllocator = LimitedAllocator;

/// Runs `f` with allocations of `bytes` or more failing on this thread.
fn with_limit<T>(bytes: usize, f: impl FnOnce() -> T) -> T {
    LIMIT.with(|cell| cell.set(bytes));
    let result = f();
    LIMIT.with(|cell| cell.set(usize::MAX));
    result
}

#[test]
fn mqtt_encode_reports_a_refused_packet_buffer() {
    // A 20 KiB payload fits; its encoded copy does not, as on a 32 KiB heap.
    let publish = MqttPublish::reliable(
        "sensors/oom",
        vec![0x5a; 20 * 1024],
        MqttQos::AtLeastOnce,
        1,
    )
    .unwrap();
    assert_eq!(
        with_limit(20 * 1024, || publish.encode()),
        Err(MqttCodecError::AllocationFailed)
    );
    // The builder is unchanged and encodes once memory is available.
    let packet = publish.encode().unwrap();
    assert_eq!(packet.len(), 1 + 3 + 2 + 11 + 2 + 1 + 20 * 1024);
}

#[test]
fn coap_encode_and_path_segments_report_refused_buffers() {
    let request = CoapRequest::new(CoapMessageType::Confirmable, CoapMethod::Post, 1, [1])
        .and_then(|request| request.path_segment("telemetry"))
        .and_then(|request| request.payload(vec![0; 600]))
        .unwrap();
    assert_eq!(
        with_limit(600, || request.encode()),
        Err(CoapCodecError::AllocationFailed)
    );
    assert_eq!(request.encode().map(|datagram| datagram.len()), Ok(616));

    // Growing the segment list allocates too; the segment is built first.
    let segment = String::from("state");
    let empty = CoapRequest::new(CoapMessageType::Confirmable, CoapMethod::Get, 1, [1]).unwrap();
    assert_eq!(
        with_limit(1, || empty.path_segment(segment)),
        Err(CoapCodecError::AllocationFailed)
    );
}

#[test]
fn ota_verification_reports_a_refused_signing_buffer() {
    let key = SigningKey::from_bytes(&[11; 32]);
    let firmware = b"firmware image";
    let manifest = OtaManifest::from_firmware("board", "1.1.0", 2, firmware).unwrap();
    let signature = key.sign(&manifest.signing_bytes().unwrap()).to_bytes();
    let mut ota =
        OtaManager::new_with_trusted_key("board", "1.0.0", 1, key.verifying_key().to_bytes())
            .unwrap();

    assert_eq!(
        with_limit(64, || manifest.signing_bytes()),
        Err(OtaError::AllocationFailed)
    );
    assert_eq!(
        with_limit(64, || ota.verify_update(&manifest, firmware, &signature)),
        Err(OtaError::AllocationFailed)
    );
    assert_eq!(ota.status(), OtaStatus::Failed);
    assert!(ota.pending_manifest().is_none());

    // With memory available the same update verifies.
    ota.verify_update(&manifest, firmware, &signature).unwrap();
    assert_eq!(ota.status(), OtaStatus::Verified);
}
