//! Cross-platform GPIO abstraction for embedded targets.

/// GPIO Pin operating mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinMode {
    Input,
    Output,
    Analog,
}

/// GPIO Pin logic state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinState {
    Low = 0,
    High = 1,
}

/// Simulated / Hardware GPIO Pin controller.
pub struct GpioPin {
    pub pin_number: u8,
    pub mode: PinMode,
    state: PinState,
}

impl GpioPin {
    /// Creates a new GpioPin instance.
    pub fn new(pin_number: u8, mode: PinMode) -> Self {
        Self {
            pin_number,
            mode,
            state: PinState::Low,
        }
    }

    /// Drives the pin output HIGH.
    pub fn set_high(&mut self) {
        self.state = PinState::High;
    }

    /// Drives the pin output LOW.
    pub fn set_low(&mut self) {
        self.state = PinState::Low;
    }

    /// Reads current logic state of the pin.
    pub fn read(&self) -> PinState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpio_pin_toggling() {
        let mut pin = GpioPin::new(13, PinMode::Output);
        assert_eq!(pin.read(), PinState::Low);
        pin.set_high();
        assert_eq!(pin.read(), PinState::High);
        pin.set_low();
        assert_eq!(pin.read(), PinState::Low);
    }
}
