//! Records where a request task panicked, at the moment it panics.
//!
//! A backtrace taken after `JoinHandle` reports the panic describes the
//! catching task, not the panic site. The console therefore installs one
//! chained panic hook that, only inside a task spawned by [`spawn_capturing`],
//! stores the panic location and (when `RUST_BACKTRACE` enables it) the
//! backtrace of the panicking thread. Panics elsewhere are passed to the
//! previous hook untouched.

use std::backtrace::{Backtrace, BacktraceStatus};
use std::future::Future;
use std::sync::{Arc, Mutex, Once};

/// Panic site of one request task.
#[derive(Debug, Default)]
pub(crate) struct PanicCapture {
    /// Source file and line reported by the panic itself.
    pub(crate) location: Option<(String, u32)>,
    /// Backtrace of the panicking thread in `Display` form, when captured.
    pub(crate) backtrace: Option<String>,
}

type Slot = Arc<Mutex<Option<PanicCapture>>>;

tokio::task_local! {
    static PANIC_SLOT: Slot;
}

static HOOK: Once = Once::new();

fn install_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = PANIC_SLOT.try_with(|slot| {
                let backtrace = Backtrace::capture();
                let capture = PanicCapture {
                    location: info
                        .location()
                        .map(|location| (location.file().to_string(), location.line())),
                    backtrace: (backtrace.status() == BacktraceStatus::Captured)
                        .then(|| backtrace.to_string()),
                };
                if let Ok(mut slot) = slot.lock()
                    && slot.is_none()
                {
                    *slot = Some(capture);
                }
            });
            previous(info);
        }));
    });
}

/// Receives the panic site recorded for one task of [`spawn_capturing`].
pub(crate) struct PanicSlot(Slot);

impl PanicSlot {
    /// Takes the recorded panic site; empty when none was recorded.
    pub(crate) fn take(&self) -> PanicCapture {
        self.0
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .unwrap_or_default()
    }
}

/// Spawns `future` on Tokio and records the site of a panic inside it.
pub(crate) fn spawn_capturing<F>(future: F) -> (tokio::task::JoinHandle<F::Output>, PanicSlot)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    install_hook();
    let slot = Slot::default();
    let handle = tokio::spawn(PANIC_SLOT.scope(Arc::clone(&slot), future));
    (handle, PanicSlot(slot))
}
