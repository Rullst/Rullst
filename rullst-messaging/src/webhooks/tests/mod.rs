//! Unit tests for webhook signatures, destination policy and the outbox.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod config;
mod outbox;
mod signature;

use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::sync::atomic::{AtomicI64, Ordering};

const START: i64 = 1_800_000_000_000;

/// A trusted clock that only moves when the test moves it.
#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    fn at(millis: i64) -> Self {
        Self(Arc::new(AtomicI64::new(millis)))
    }

    fn set(&self, millis: i64) {
        self.0.store(millis, Ordering::SeqCst);
    }

    fn advance(&self, millis: i64) {
        self.0.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_millis(&self) -> crate::Result<i64> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

fn live_key() -> WebhookSigningKey {
    WebhookSigningKey::new("fixture", URL_SAFE_NO_PAD.encode([37; 32])).unwrap()
}

#[test]
fn trusted_time_ends_a_week_before_the_last_representable_second() {
    let clock = ManualClock::at(253_401_695_999_000);
    assert_eq!(now(&clock), Ok(253_401_695_999_000));
    clock.advance(1);
    assert_eq!(now(&clock), Err(WebhookError::Clock));
    clock.set(0);
    assert_eq!(now(&clock), Ok(0));
    clock.set(-1);
    assert_eq!(now(&clock), Err(WebhookError::Clock));
}
