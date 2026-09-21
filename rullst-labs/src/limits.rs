use crate::LabError;
use serde::{Deserialize, Serialize};

pub const MAX_SOURCE_BYTES: usize = 32_768;
pub const MAX_WASM_BYTES: usize = 262_144;
pub const MAX_CASES: usize = 64;
pub const MAX_DIAGNOSTIC_BYTES: usize = 8192;

/// A student cannot raise these limits or enable imports/network/shell access.
/// Construction validates configuration; it does not prove enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "LimitsWire", into = "LimitsWire")]
pub struct ExecutionLimits {
    wall_seconds: u32,
    fuel_per_case: u64,
    memory_pages: u32,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LimitsWire {
    wall_seconds: u32,
    fuel_per_case: u64,
    memory_pages: u32,
}
impl ExecutionLimits {
    pub fn new(wall_seconds: u32, fuel_per_case: u64, memory_pages: u32) -> Result<Self, LabError> {
        if !(5..=60).contains(&wall_seconds)
            || !(1000..=1_000_000).contains(&fuel_per_case)
            || !(32..=256).contains(&memory_pages)
        {
            return Err(LabError::InvalidInput);
        }
        Ok(Self {
            wall_seconds,
            fuel_per_case,
            memory_pages,
        })
    }
    pub fn wall_seconds(&self) -> u32 {
        self.wall_seconds
    }
    pub fn fuel_per_case(&self) -> u64 {
        self.fuel_per_case
    }
    pub fn memory_pages(&self) -> u32 {
        self.memory_pages
    }
}
impl TryFrom<LimitsWire> for ExecutionLimits {
    type Error = LabError;
    fn try_from(value: LimitsWire) -> Result<Self, LabError> {
        Self::new(value.wall_seconds, value.fuel_per_case, value.memory_pages)
    }
}
impl From<ExecutionLimits> for LimitsWire {
    fn from(value: ExecutionLimits) -> Self {
        Self {
            wall_seconds: value.wall_seconds,
            fuel_per_case: value.fuel_per_case,
            memory_pages: value.memory_pages,
        }
    }
}
