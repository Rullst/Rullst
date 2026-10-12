//! Radar's process probes and boot clock. They read process-wide state (the
//! boot time and the previous CPU sample), so they run in their own binary.

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::radar::{RadarSnapshot, get_process_memory_mb, init_radar};
use std::time::{Duration, Instant};

/// `VmRSS` of this process in MiB, read independently of Radar.
fn vm_rss_mib() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let kib: f64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .expect("VmRSS line");
    kib / 1024.0
}

#[tokio::test]
async fn probes_report_real_memory_cpu_tasks_and_uptime() {
    // Memory is VmRSS in MiB, rounded to one decimal.
    let expected = vm_rss_mib();
    let reported = get_process_memory_mb().expect("Linux memory probe");
    assert!(
        (reported - expected).abs() <= expected * 0.25 + 1.0,
        "reported {reported} MiB, VmRSS {expected} MiB"
    );

    // Spawned tasks are counted while they are alive.
    let (release, released) = tokio::sync::watch::channel(false);
    for _ in 0..3 {
        let mut released = released.clone();
        tokio::spawn(async move {
            let _ = released.wait_for(|done| *done).await;
        });
    }

    // Before `init_radar`, uptime comes from the process clock.
    let first = RadarSnapshot::collect();
    assert!(first.uptime_seconds < 2, "{}", first.uptime_seconds);
    assert!(first.active_tokio_tasks.is_some_and(|tasks| tasks >= 3));
    let _ = release.send(true);

    tokio::time::sleep(Duration::from_millis(2_200)).await;
    init_radar();
    tokio::time::sleep(Duration::from_millis(2_200)).await;

    // Burn some CPU so the next sample has a measurable process share.
    let started = Instant::now();
    let mut spin = 0_u64;
    while started.elapsed() < Duration::from_millis(300) {
        spin = std::hint::black_box(spin.wrapping_add(1));
    }

    // Uptime now counts from `init_radar`, not from the first snapshot.
    let second = RadarSnapshot::collect();
    assert!(
        (2..=3).contains(&second.uptime_seconds),
        "{}",
        second.uptime_seconds
    );
    let cpu = second.cpu_usage_percent.expect("a second CPU sample");
    assert!(cpu > 1.0, "{cpu}");
}
