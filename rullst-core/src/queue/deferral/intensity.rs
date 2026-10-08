//! Pluggable carbon-intensity forecasts.
//!
//! Core ships no network implementation. Applications implement
//! [`CarbonIntensitySource`] for their provider; [`FixedIntensitySource`] is a
//! deterministic, offline source for tests and documentation.

use async_trait::async_trait;
use std::time::SystemTime;

/// A failure reported by a [`CarbonIntensitySource`]. Unpublished v13 API.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum IntensitySourceError {
    /// The source could not produce a forecast (network, quota, parsing...).
    #[error("carbon intensity source unavailable: {0}")]
    Unavailable(String),
}

impl IntensitySourceError {
    /// Creates an [`Self::Unavailable`] error.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable(reason.into())
    }
}

/// The forecast intensity of one time slot, `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct IntensitySlot {
    /// Inclusive start of the slot.
    pub start: SystemTime,
    /// Exclusive end of the slot.
    pub end: SystemTime,
    /// Forecast intensity, in the forecast's [`IntensityForecast::unit`].
    pub value: f64,
}

/// Intensity values for consecutive or sparse time slots, in one unit.
///
/// Slots with a non-finite or negative value, or whose end is not after their
/// start, are ignored by the planner. Unpublished v13 API.
#[derive(Debug, Clone, PartialEq)]
pub struct IntensityForecast {
    unit: String,
    slots: Vec<IntensitySlot>,
}

impl IntensityForecast {
    /// Creates an empty forecast whose values use `unit`, for example
    /// `"gCO2eq/kWh"`.
    pub fn new(unit: impl Into<String>) -> Self {
        Self {
            unit: unit.into(),
            slots: Vec::new(),
        }
    }

    /// Adds the slot `[start, end)` with the given intensity value.
    pub fn slot(mut self, start: SystemTime, end: SystemTime, value: f64) -> Self {
        self.slots.push(IntensitySlot { start, end, value });
        self
    }

    /// The unit of every value in this forecast.
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// The forecast slots, in the order the source returned them.
    pub fn slots(&self) -> &[IntensitySlot] {
        &self.slots
    }
}

/// A provider of carbon-intensity forecasts used to place deferrable jobs.
///
/// Implementations may call a remote API; the planner bounds each call with
/// its timeout and falls back to the time-window rule on error or timeout.
/// Core ships no network implementation. Unpublished v13 API.
#[async_trait]
pub trait CarbonIntensitySource: Send + Sync {
    /// Short, stable name recorded with every placement that consulted the
    /// source, such as `"grid-operator-forecast"`.
    fn name(&self) -> &str;

    /// Returns forecast slots overlapping `[from, until)`.
    async fn forecast(
        &self,
        from: SystemTime,
        until: SystemTime,
    ) -> Result<IntensityForecast, IntensitySourceError>;
}

/// A deterministic source that answers from a fixed list of slots and never
/// performs I/O. Intended for tests, examples and documentation.
/// Unpublished v13 API.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedIntensitySource {
    name: String,
    forecast: IntensityForecast,
}

impl FixedIntensitySource {
    /// Creates a source named `name` whose values use `unit`.
    pub fn new(name: impl Into<String>, unit: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            forecast: IntensityForecast::new(unit),
        }
    }

    /// Adds the slot `[start, end)` with the given intensity value.
    pub fn slot(mut self, start: SystemTime, end: SystemTime, value: f64) -> Self {
        self.forecast = self.forecast.slot(start, end, value);
        self
    }
}

#[async_trait]
impl CarbonIntensitySource for FixedIntensitySource {
    fn name(&self) -> &str {
        &self.name
    }

    async fn forecast(
        &self,
        from: SystemTime,
        until: SystemTime,
    ) -> Result<IntensityForecast, IntensitySourceError> {
        let mut forecast = IntensityForecast::new(self.forecast.unit());
        for slot in self.forecast.slots() {
            if slot.end > from && slot.start < until {
                forecast = forecast.slot(slot.start, slot.end, slot.value);
            }
        }
        Ok(forecast)
    }
}
