//! Voltage-based power-mode recommendation helpers.

/// Power governor operating modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerMode {
    /// Recommendation for full execution.
    FullActive,
    /// Recommendation for reduced power use.
    LowPower,
    /// Recommendation for platform-specific deep sleep.
    DeepSleep,
}

/// Solar harvester voltage state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarvesterState {
    Charging,
    Full,
    Low,
    Critical,
}

/// Embedded power consumption governor.
pub struct PowerGovernor {
    pub mode: PowerMode,
    pub battery_mv: u32,
    pub solar_mv: u32,
}

impl PowerGovernor {
    pub fn new(battery_mv: u32, solar_mv: u32) -> Self {
        Self {
            mode: PowerMode::FullActive,
            battery_mv,
            solar_mv,
        }
    }

    /// Evaluates current energy budget and recommends a power mode.
    pub fn evaluate(&mut self) -> PowerMode {
        let recommended = match self.battery_mv {
            0..=2800 => PowerMode::DeepSleep,
            2801..=3300 => PowerMode::LowPower,
            _ => PowerMode::FullActive,
        };
        self.mode = recommended;
        recommended
    }

    /// Checks solar harvester voltage state.
    pub fn harvester_state(&self) -> HarvesterState {
        match self.solar_mv {
            0..=500 => HarvesterState::Critical,
            501..=2000 => HarvesterState::Low,
            2001..=3800 => HarvesterState::Charging,
            _ => HarvesterState::Full,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_governor_modes() {
        let mut gov = PowerGovernor::new(2700, 3600);
        assert_eq!(gov.evaluate(), PowerMode::DeepSleep);

        gov.battery_mv = 3100;
        assert_eq!(gov.evaluate(), PowerMode::LowPower);

        gov.battery_mv = 4200;
        assert_eq!(gov.evaluate(), PowerMode::FullActive);
    }

    #[test]
    fn test_harvester_state() {
        let gov = PowerGovernor::new(3700, 3900);
        assert_eq!(gov.harvester_state(), HarvesterState::Full);

        let gov2 = PowerGovernor::new(3700, 400);
        assert_eq!(gov2.harvester_state(), HarvesterState::Critical);
    }
}
