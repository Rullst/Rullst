#![cfg(feature = "sqlite")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! TEMP-DIAG: Windows SQLite flake investigation; removed before the final commit.
use rullst_labs::{sqlite::*, *};

fn env(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[test]
#[ignore = "Windows flake investigation"]
fn temp_stress_initialize() {
    let iterations = env("STRESS_ITERATIONS", 200);
    let threads = env("STRESS_THREADS", 4);
    let handles: Vec<_> = (0..threads)
        .map(|thread| {
            std::thread::Builder::new()
                .name(format!("stress-{thread}"))
                .spawn(move || {
                    let mut failures = 0usize;
                    for iteration in 0..iterations {
                        // A fresh runtime per iteration, like one #[tokio::test].
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        let directory = tempfile::tempdir().unwrap();
                        let path = directory.path().join("labs.sqlite");
                        let started = std::time::Instant::now();
                        let result = runtime.block_on(async {
                            SqliteLabs::initialize(
                                &path,
                                StoreConfig::new(
                                    Reference::new("stress").unwrap(),
                                    10,
                                    4,
                                    ExecutionProfile::Simulation,
                                )
                                .unwrap(),
                                ContentKey::new([9; 32]).unwrap(),
                                SystemClock,
                            )
                            .await
                        });
                        let elapsed = started.elapsed().as_millis();
                        if elapsed > 1500 {
                            eprintln!(
                                "STRESS-SLOW thread={thread} iteration={iteration} ms={elapsed} ok={}",
                                result.is_ok()
                            );
                        }
                        match result {
                            Ok(store) => {
                                if iteration % 2 == 0 {
                                    runtime.block_on(store.close());
                                }
                                drop(store);
                            }
                            Err(error) => {
                                failures += 1;
                                eprintln!(
                                    "STRESS-FAIL thread={thread} iteration={iteration} after_ms={} error={error:?}",
                                    started.elapsed().as_millis()
                                );
                            }
                        }
                        drop(runtime);
                    }
                    failures
                })
                .unwrap()
        })
        .collect();
    let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    eprintln!("STRESS-TOTAL failures={total} of {}", threads * iterations);
}
