use crate::SupervisionError as Error;

/// Disclosed categories, not proof of OS/browser permission or lawful processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Capability {
    Visibility,
    WindowFocus,
    ClipboardActivity,
    Fullscreen,
    CameraStatus,
    MicrophoneStatus,
    ScreenShareStatus,
    CameraPresence,
    AudioActivity,
}

impl Capability {
    pub const ALL: [Self; 9] = [
        Self::Visibility,
        Self::WindowFocus,
        Self::ClipboardActivity,
        Self::Fullscreen,
        Self::CameraStatus,
        Self::MicrophoneStatus,
        Self::ScreenShareStatus,
        Self::CameraPresence,
        Self::AudioActivity,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Visibility => "visibility",
            Self::WindowFocus => "focus",
            Self::ClipboardActivity => "clipboard",
            Self::Fullscreen => "fullscreen",
            Self::CameraStatus => "camera_status",
            Self::MicrophoneStatus => "microphone_status",
            Self::ScreenShareStatus => "screen_share_status",
            Self::CameraPresence => "camera_presence",
            Self::AudioActivity => "audio_activity",
        }
    }
    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// Immutable bounded category selection. Unknown/duplicate categories are errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Collection(u16);

impl Collection {
    pub fn new(capabilities: impl IntoIterator<Item = Capability>) -> Result<Self, Error> {
        let mut bits = 0;
        for (index, capability) in capabilities.into_iter().enumerate() {
            if index >= Capability::ALL.len() || bits & capability.bit() != 0 {
                return Err(Error::InvalidInput);
            }
            bits |= capability.bit();
        }
        Ok(Self(bits))
    }
    pub const fn visibility_only() -> Self {
        Self(1)
    }
    pub const fn none() -> Self {
        Self(0)
    }
    pub fn from_bits(bits: u16) -> Result<Self, Error> {
        if bits & !511 != 0 {
            return Err(Error::InvalidInput);
        }
        Ok(Self(bits))
    }
    pub const fn bits(self) -> u16 {
        self.0
    }
    pub const fn contains(self, capability: Capability) -> bool {
        self.0 & capability.bit() != 0
    }
    pub const fn is_subset_of(self, previous: Self) -> bool {
        self.0 & !previous.0 == 0
    }
    pub fn capabilities(self) -> impl Iterator<Item = Capability> {
        Capability::ALL
            .into_iter()
            .filter(move |capability| self.contains(*capability))
    }
}
