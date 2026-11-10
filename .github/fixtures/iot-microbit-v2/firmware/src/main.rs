//! On-hardware test suite for `rullst-iot` with `default-features = false`
//! on a BBC micro:bit V2 (nRF52833, Cortex-M4F, 64 MHz, 128 KiB RAM).
//!
//! CI builds it for `thumbv7em-none-eabihf` so that a `no_std` regression in
//! `rullst-iot` fails (`iot-integration.yml`). To run it on a board:
//!
//! ```console
//! cd .github/fixtures/iot-microbit-v2/firmware
//! cargo build --release --locked
//! rust-objcopy -O ihex target/thumbv7em-none-eabihf/release/rullst-iot-microbit-v2 fw.hex
//! cp fw.hex /media/$USER/MICROBIT/      # DAPLink flashes and resets the board
//! ```
//!
//! and read `/dev/ttyACM0` at 115200 8N1 (UARTE0, TX P0.06, bridged by the
//! DAPLink interface chip): one `PASS|FAIL <name> <detail>` line per check,
//! `INFO` lines with timings (DWT cycle counter) and heap/stack use, a
//! `SUMMARY` per run and a final `END total_pass=<n> total_fail=<m>`. A serial
//! break resets the board for another run. The expected bytes in
//! `src/expected.rs` come from the `std` build of `rullst-iot`:
//! `cargo run --locked --manifest-path ../vectors/Cargo.toml` rewrites them,
//! and CI checks that they are current.

#![no_std]
#![no_main]

extern crate alloc;

#[allow(dead_code)]
#[rustfmt::skip]
mod expected;
mod store;
mod suite;

use core::{
    alloc::{GlobalAlloc, Layout},
    cell::RefCell,
    fmt::{self, Write},
    mem::MaybeUninit,
    ptr::{addr_of, addr_of_mut},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};
use cortex_m::peripheral::DWT;
use cortex_m_rt::entry;
use critical_section::Mutex;
use embedded_alloc::LlffHeap;
use microbit::{
    Board,
    hal::{
        clocks::Clocks,
        timer::Timer,
        uarte::{Baudrate, Parity, Uarte},
    },
    pac,
};

/// Heap reserved for rullst-iot's `alloc` use.
pub const HEAP_SIZE: usize = 32 * 1024;
const STACK_PAINT: u32 = 0xC0DE_CAFE;
/// Bytes below the live stack pointer left unpainted at boot.
const PAINT_MARGIN: usize = 256;
pub const CPU_HZ: u32 = 64_000_000;

/// Linked-list heap that also records live bytes, peak bytes and failures.
pub struct TrackingHeap {
    inner: LlffHeap,
    used: AtomicUsize,
    peak: AtomicUsize,
    allocations: AtomicUsize,
    failures: AtomicUsize,
}

impl TrackingHeap {
    const fn new() -> Self {
        Self {
            inner: LlffHeap::empty(),
            used: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            allocations: AtomicUsize::new(0),
            failures: AtomicUsize::new(0),
        }
    }
    pub fn used(&self) -> usize {
        self.used.load(Relaxed)
    }
    pub fn peak(&self) -> usize {
        self.peak.load(Relaxed)
    }
    pub fn allocations(&self) -> usize {
        self.allocations.load(Relaxed)
    }
    pub fn failures(&self) -> usize {
        self.failures.load(Relaxed)
    }
    /// Restarts the peak at the current live size (per-group high-water mark).
    pub fn reset_peak(&self) {
        self.peak.store(self.used.load(Relaxed), Relaxed);
    }
}

unsafe impl GlobalAlloc for TrackingHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.inner.alloc(layout) };
        if ptr.is_null() {
            self.failures.fetch_add(1, Relaxed);
        } else {
            let now = self.used.fetch_add(layout.size(), Relaxed) + layout.size();
            self.peak.fetch_max(now, Relaxed);
            self.allocations.fetch_add(1, Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.dealloc(ptr, layout) };
        self.used.fetch_sub(layout.size(), Relaxed);
    }
}

#[global_allocator]
pub static HEAP: TrackingHeap = TrackingHeap::new();
static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

static UART: Mutex<RefCell<Option<Uarte<pac::UARTE0>>>> = Mutex::new(RefCell::new(None));

/// Writes one CRLF-terminated line to the DAPLink serial bridge.
pub fn emit(args: fmt::Arguments) {
    critical_section::with(|cs| {
        if let Ok(mut slot) = UART.borrow(cs).try_borrow_mut()
            && let Some(uart) = slot.as_mut()
        {
            let _ = uart.write_fmt(args);
            let _ = uart.write_str("\r\n");
        }
    });
}

#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => { $crate::emit(format_args!($($arg)*)) };
}

/// Runs `f` and returns its result with the elapsed DWT cycles.
pub fn timed<T>(f: impl FnOnce() -> T) -> (T, u32) {
    let start = DWT::cycle_count();
    let value = f();
    (value, DWT::cycle_count().wrapping_sub(start))
}

/// Milliseconds with two decimals for a 64 MHz cycle count.
pub struct Ms(pub u32);
impl fmt::Display for Ms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hundredths = (u64::from(self.0) * 100) / u64::from(CPU_HZ / 1000);
        write!(f, "{}.{:02}ms", hundredths / 100, hundredths % 100)
    }
}

unsafe extern "C" {
    static mut __sheap: u32;
    static _stack_start: u32;
}

/// Paints the free RAM between the end of statics and the live stack pointer.
#[inline(never)]
fn paint_stack() -> (usize, usize) {
    let bottom = addr_of_mut!(__sheap) as usize;
    let sp = cortex_m::register::msp::read() as usize;
    let limit = sp - PAINT_MARGIN;
    let mut address = bottom;
    while address < limit {
        unsafe { core::ptr::write_volatile(address as *mut u32, STACK_PAINT) };
        address += 4;
    }
    (bottom, sp)
}

/// Paints the free stack below the caller, runs `f`, and returns the deepest
/// stack use of `f` (and its callees) in bytes, at 16-byte resolution.
#[inline(never)]
pub fn stack_depth<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let bottom = addr_of_mut!(__sheap) as usize;
    let sp = cortex_m::register::msp::read() as usize;
    let limit = sp - 16;
    let mut address = bottom;
    while address < limit {
        unsafe { core::ptr::write_volatile(address as *mut u32, STACK_PAINT) };
        address += 4;
    }
    let value = f();
    let deepest = stack_high_water();
    DEEPEST.fetch_min(deepest, Relaxed);
    (value, sp.saturating_sub(deepest))
}

/// Deepest stack address observed by any `stack_depth` call since boot.
static DEEPEST: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Lowest stack address that is no longer painted, i.e. the deepest use.
fn stack_high_water() -> usize {
    let bottom = addr_of_mut!(__sheap) as usize;
    let top = addr_of!(_stack_start) as usize;
    let mut address = bottom;
    while address < top {
        if unsafe { core::ptr::read_volatile(address as *const u32) } != STACK_PAINT {
            return address;
        }
        address += 4;
    }
    top
}

#[entry]
fn main() -> ! {
    let (paint_bottom, boot_sp) = paint_stack();
    // SAFETY: called once, before any allocation, on a region used only by HEAP.
    unsafe { HEAP.inner.init(addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) };

    let Some(mut board) = Board::take() else {
        loop {
            cortex_m::asm::wfi();
        }
    };
    let _clocks = Clocks::new(board.CLOCK).enable_ext_hfosc();
    let uart = Uarte::new(
        board.UARTE0,
        board.uart.into(),
        Parity::EXCLUDED,
        Baudrate::BAUD115200,
    );
    critical_section::with(|cs| UART.borrow(cs).replace(Some(uart)));
    board.DCB.enable_trace();
    board.DWT.enable_cycle_counter();
    let mut timer = Timer::new(board.TIMER0);
    timer.start(u32::MAX);

    // Give the host a moment to (re)open the CDC port after reset.
    let wait_start = timer.read();
    while timer.read().wrapping_sub(wait_start) < 1_500_000 {}

    let top = addr_of!(_stack_start) as usize;
    let ficr = unsafe { &*pac::FICR::ptr() };
    outln!("");
    outln!(
        "BOOT rullst-iot-microbit-v2 profile={} ed25519_fast={} part=0x{:x} variant=0x{:08x} ram={}KiB flash={}KiB deviceid={:08x}{:08x}",
        env!("FW_PROFILE"),
        cfg!(feature = "ed25519-fast"),
        ficr.info.part.read().bits(),
        ficr.info.variant.read().bits(),
        ficr.info.ram.read().bits(),
        ficr.info.flash.read().bits(),
        ficr.deviceid[1].read().bits(),
        ficr.deviceid[0].read().bits(),
    );
    outln!(
        "INFO layout heap={}B statics_end=0x{:08x} stack_top=0x{:08x} stack_region={}B boot_sp=0x{:08x}",
        HEAP_SIZE,
        paint_bottom,
        top,
        top - paint_bottom,
        boot_sp
    );

    // Cross-check the 64 MHz DWT assumption against TIMER0 (1 MHz, HFXO).
    let t0 = timer.read();
    let c0 = DWT::cycle_count();
    while timer.read().wrapping_sub(t0) < 200_000 {}
    let cycles = DWT::cycle_count().wrapping_sub(c0);
    let micros = timer.read().wrapping_sub(t0);
    outln!(
        "INFO clock dwt_cycles={} timer_us={} cpu_mhz~{}",
        cycles,
        micros,
        cycles / micros.max(1)
    );

    // Boot-to-here stack use, before the per-call probes repaint the stack.
    let boot_water = stack_high_water();
    outln!("INFO stack boot_used={}B", top - boot_water);

    let mut totals = (0u32, 0u32);
    for run in 1..=2u32 {
        outln!("BEGIN run={}", run);
        let mut ctx = suite::Ctx::default();
        // Whole-suite stack use measured first (fresh paint), then the per-call probes.
        let ((), depth) = stack_depth(|| suite::run(&mut ctx));
        outln!("INFO stack suite_run_depth={}B run={}", depth, run);
        suite::stack_probes();
        outln!("SUMMARY pass={} fail={} run={}", ctx.pass, ctx.fail, run);
        totals.0 += ctx.pass;
        totals.1 += ctx.fail;
    }

    let suite_deepest = DEEPEST.load(Relaxed).min(boot_water);
    outln!(
        "INFO stack used={}B of {}B (deepest=0x{:08x}, region from 0x{:08x})",
        top - suite_deepest,
        top - paint_bottom,
        suite_deepest,
        paint_bottom
    );
    outln!(
        "INFO heap peak={}B live_now={}B allocations={} failures={} capacity={}B",
        HEAP.peak_all(),
        HEAP.used(),
        HEAP.allocations(),
        HEAP.failures(),
        HEAP_SIZE
    );
    // After the memory report, so that its 20 KiB payload is not in the peak.
    let mut allocation = suite::Ctx::default();
    suite::allocation_failure_checks(&mut allocation);
    outln!(
        "SUMMARY pass={} fail={} run=allocation",
        allocation.pass,
        allocation.fail
    );
    totals.0 += allocation.pass;
    totals.1 += allocation.fail;
    outln!("END total_pass={} total_fail={}", totals.0, totals.1);

    loop {
        cortex_m::asm::wfi();
    }
}

impl TrackingHeap {
    /// Overall peak since boot (kept separately from the per-group peak).
    pub fn peak_all(&self) -> usize {
        BOOT_PEAK.load(Relaxed).max(self.peak())
    }
    /// Folds the current group peak into the since-boot peak, then resets it.
    pub fn start_group(&self) {
        BOOT_PEAK.fetch_max(self.peak(), Relaxed);
        self.reset_peak();
    }
}
static BOOT_PEAK: AtomicUsize = AtomicUsize::new(0);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    emit(format_args!("PANIC {}", info));
    loop {
        cortex_m::asm::wfi();
    }
}
