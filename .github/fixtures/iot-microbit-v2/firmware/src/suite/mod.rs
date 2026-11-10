//! The rullst-iot test cases. Every case prints `PASS|FAIL <name> <detail>`.

mod cas;
mod frames;
mod ota;
mod probes;
mod protocols;

pub use probes::{allocation_failure_checks, stack_probes};

use crate::{HEAP, outln, store::RamCounterStore};
use core::fmt;
use rullst_iot::{BootPartition, OtaError, OtaManager, OtaManifest};

/// The synthetic 32 KiB image `build.rs` writes (see `image.rs`).
static FIRMWARE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/firmware.bin"));
const TARGET: &str = "microbit-v2";

#[derive(Default)]
pub struct Ctx {
    pub pass: u32,
    pub fail: u32,
}

impl Ctx {
    pub(crate) fn check(&mut self, name: &str, ok: bool, detail: fmt::Arguments) {
        if ok {
            self.pass += 1;
            outln!("PASS {} {}", name, detail);
        } else {
            self.fail += 1;
            outln!("FAIL {} {}", name, detail);
        }
    }

    pub(crate) fn bytes(&mut self, name: &str, actual: &[u8], expected: &[u8]) {
        match actual.iter().zip(expected).position(|(a, e)| a != e) {
            None if actual.len() == expected.len() => self.check(
                name,
                true,
                format_args!("len={} identical to host", actual.len()),
            ),
            None => self.check(
                name,
                false,
                format_args!("len={} host_len={}", actual.len(), expected.len()),
            ),
            Some(i) => self.check(
                name,
                false,
                format_args!(
                    "first_diff@{} got=0x{:02x} host=0x{:02x} len={}/{}",
                    i,
                    actual[i],
                    expected[i],
                    actual.len(),
                    expected.len()
                ),
            ),
        }
    }

    pub(crate) fn expect_err<T, E: PartialEq + fmt::Debug>(
        &mut self,
        name: &str,
        result: Result<T, E>,
        want: E,
    ) {
        match result {
            Err(e) if e == want => self.check(name, true, format_args!("rejected {:?}", e)),
            Err(e) => self.check(name, false, format_args!("got {:?} want {:?}", e, want)),
            Ok(_) => self.check(name, false, format_args!("accepted, want {:?}", want)),
        }
    }
}

fn heap_group(_name: &str) {
    HEAP.start_group();
}

fn heap_report(name: &str) {
    outln!(
        "INFO heap_peak group={} bytes={} live={}",
        name,
        HEAP.peak(),
        HEAP.used()
    );
}

pub fn run(ctx: &mut Ctx) {
    heap_group("frames");
    frames::frames(ctx);
    heap_report("frames");
    heap_group("mqtt");
    protocols::mqtt(ctx);
    heap_report("mqtt");
    heap_group("coap");
    protocols::coap(ctx);
    heap_report("coap");
    heap_group("anomaly");
    protocols::anomaly(ctx);
    heap_report("anomaly");
    heap_group("ota");
    ota::ota(ctx);
    heap_report("ota");
    heap_group("cas");
    cas::cas(ctx);
    heap_report("cas");
}

fn key(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(bytes);
    out
}

fn manifest(
    target: &str,
    version: &str,
    counter: u64,
    firmware: &[u8],
) -> Result<OtaManifest, OtaError> {
    OtaManifest::from_firmware(target, version, counter, firmware)
}

fn manager(
    store: &mut RamCounterStore,
    public_key: &[u8],
    running: BootPartition,
) -> Result<OtaManager, OtaError> {
    OtaManager::new_with_running_partition(TARGET, "1.0.0", running, key(public_key), store)
}
