//! In-RAM `RollbackCounterStore` following the documented contract, with
//! fault injection for the outcome-unknown and unavailable cases.

use rullst_iot::{RollbackCounterError, RollbackCounterStore};

pub struct RamCounterStore {
    pub value: u64,
    pub writes: u32,
    /// The next `load` fails with this error.
    pub fail_next_load: Option<RollbackCounterError>,
    /// The next successful write reports this error (lost acknowledgement).
    pub lose_next_ack: Option<RollbackCounterError>,
}

impl RamCounterStore {
    pub const fn new(value: u64) -> Self {
        Self {
            value,
            writes: 0,
            fail_next_load: None,
            lose_next_ack: None,
        }
    }
}

impl RollbackCounterStore for RamCounterStore {
    fn load(&mut self) -> Result<u64, RollbackCounterError> {
        if let Some(error) = self.fail_next_load.take() {
            return Err(error);
        }
        Ok(self.value)
    }

    fn compare_and_set(
        &mut self,
        expected: u64,
        proposed: u64,
    ) -> Result<(), RollbackCounterError> {
        if self.value != expected {
            return Err(RollbackCounterError::Conflict {
                expected,
                actual: self.value,
            });
        }
        if proposed <= self.value {
            return Err(RollbackCounterError::NonMonotonic {
                current: self.value,
                proposed,
            });
        }
        self.value = proposed;
        self.writes += 1;
        match self.lose_next_ack.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
