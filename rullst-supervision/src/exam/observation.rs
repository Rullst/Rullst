use super::Capability;
use crate::{Context, OpaqueId, Revision, Scope, SupervisionError as Error};

/// Host-authenticated binding for one exact next observation. Shape validation
/// is not authentication; the store rechecks subject/session/revision and policy.
#[derive(Clone, Copy)]
pub struct ObservationRequest<'a> {
    pub(crate) context: &'a Context,
    pub(crate) scope: &'a Scope,
    pub(crate) session: &'a OpaqueId,
    pub(crate) revision: Revision,
    pub(crate) sequence: i64,
}
impl<'a> ObservationRequest<'a> {
    pub fn context(self) -> &'a Context {
        self.context
    }
    pub fn scope(self) -> &'a Scope {
        self.scope
    }
    pub fn session(self) -> &'a OpaqueId {
        self.session
    }
    pub fn revision(self) -> Revision {
        self.revision
    }
    pub fn sequence(self) -> i64 {
        self.sequence
    }
    pub fn new(
        context: &'a Context,
        scope: &'a Scope,
        session: &'a OpaqueId,
        revision: Revision,
        sequence: i64,
    ) -> Result<Self, Error> {
        context.require_subject(scope)?;
        if sequence <= 0 {
            return Err(Error::Sequence);
        }
        Ok(Self {
            context,
            scope,
            session,
            revision,
            sequence,
        })
    }
}

/// Occurrences reported by the exam page. No text or other-window metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrowserEvent {
    PageVisible,
    PageHidden,
    WindowFocused,
    WindowBlurred,
    CopyAttempt,
    CutAttempt,
    PasteAttempt,
    FullscreenEntered,
    FullscreenExited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CaptureDevice {
    Camera,
    Microphone,
    ScreenShare,
}

/// A client claim, not independent confirmation that capture occurred or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CaptureEvent {
    Started,
    Stopped,
    PermissionDenied,
    Unavailable,
}

/// Detector output, never an identity, intent, attendance or misconduct verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresenceObservation {
    PersonDetected,
    NoPersonDetected,
    Inconclusive,
}

/// Speech activity cannot determine whether answers or outside help were received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AudioObservation {
    SpeechDetected,
    NoSpeechDetected,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Observation {
    Browser(BrowserEvent),
    Capture {
        device: CaptureDevice,
        event: CaptureEvent,
    },
    CameraPresence(PresenceObservation),
    AudioActivity(AudioObservation),
}

impl Observation {
    pub const fn capability(self) -> Capability {
        match self {
            Self::Browser(event) => match event {
                BrowserEvent::PageVisible | BrowserEvent::PageHidden => Capability::Visibility,
                BrowserEvent::WindowFocused | BrowserEvent::WindowBlurred => {
                    Capability::WindowFocus
                }
                BrowserEvent::CopyAttempt
                | BrowserEvent::CutAttempt
                | BrowserEvent::PasteAttempt => Capability::ClipboardActivity,
                BrowserEvent::FullscreenEntered | BrowserEvent::FullscreenExited => {
                    Capability::Fullscreen
                }
            },
            Self::Capture { device, .. } => match device {
                CaptureDevice::Camera => Capability::CameraStatus,
                CaptureDevice::Microphone => Capability::MicrophoneStatus,
                CaptureDevice::ScreenShare => Capability::ScreenShareStatus,
            },
            Self::CameraPresence(_) => Capability::CameraPresence,
            Self::AudioActivity(_) => Capability::AudioActivity,
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Browser(event) => match event {
                BrowserEvent::PageVisible => "page_visible",
                BrowserEvent::PageHidden => "page_hidden",
                BrowserEvent::WindowFocused => "window_focused",
                BrowserEvent::WindowBlurred => "window_blurred",
                BrowserEvent::CopyAttempt => "copy_attempt",
                BrowserEvent::CutAttempt => "cut_attempt",
                BrowserEvent::PasteAttempt => "paste_attempt",
                BrowserEvent::FullscreenEntered => "fullscreen_entered",
                BrowserEvent::FullscreenExited => "fullscreen_exited",
            },
            Self::Capture { event, .. } => match event {
                CaptureEvent::Started => "capture_started",
                CaptureEvent::Stopped => "capture_stopped",
                CaptureEvent::PermissionDenied => "capture_permission_denied",
                CaptureEvent::Unavailable => "capture_unavailable",
            },
            Self::CameraPresence(event) => match event {
                PresenceObservation::PersonDetected => "person_detected",
                PresenceObservation::NoPersonDetected => "no_person_detected",
                PresenceObservation::Inconclusive => "presence_inconclusive",
            },
            Self::AudioActivity(event) => match event {
                AudioObservation::SpeechDetected => "speech_detected",
                AudioObservation::NoSpeechDetected => "no_speech_detected",
                AudioObservation::Inconclusive => "audio_inconclusive",
            },
        }
    }
    #[cfg(feature = "sqlite")]
    pub(crate) const fn code(self) -> i64 {
        match self {
            Self::Browser(event) => event as i64 + 1,
            Self::Capture { device, event } => 10 + (device as i64) * 4 + event as i64,
            Self::CameraPresence(event) => 22 + event as i64,
            Self::AudioActivity(event) => 25 + event as i64,
        }
    }
    #[cfg(feature = "sqlite")]
    pub(crate) fn from_code(code: i64) -> Result<Self, Error> {
        const BROWSER: [BrowserEvent; 9] = [
            BrowserEvent::PageVisible,
            BrowserEvent::PageHidden,
            BrowserEvent::WindowFocused,
            BrowserEvent::WindowBlurred,
            BrowserEvent::CopyAttempt,
            BrowserEvent::CutAttempt,
            BrowserEvent::PasteAttempt,
            BrowserEvent::FullscreenEntered,
            BrowserEvent::FullscreenExited,
        ];
        const CAPTURE: [CaptureEvent; 4] = [
            CaptureEvent::Started,
            CaptureEvent::Stopped,
            CaptureEvent::PermissionDenied,
            CaptureEvent::Unavailable,
        ];
        const DEVICES: [CaptureDevice; 3] = [
            CaptureDevice::Camera,
            CaptureDevice::Microphone,
            CaptureDevice::ScreenShare,
        ];
        const PRESENCE: [PresenceObservation; 3] = [
            PresenceObservation::PersonDetected,
            PresenceObservation::NoPersonDetected,
            PresenceObservation::Inconclusive,
        ];
        const AUDIO: [AudioObservation; 3] = [
            AudioObservation::SpeechDetected,
            AudioObservation::NoSpeechDetected,
            AudioObservation::Inconclusive,
        ];
        match code {
            1..=9 => Ok(Self::Browser(BROWSER[(code - 1) as usize])),
            10..=21 => Ok(Self::Capture {
                device: DEVICES[((code - 10) / 4) as usize],
                event: CAPTURE[((code - 10) % 4) as usize],
            }),
            22..=24 => Ok(Self::CameraPresence(PRESENCE[(code - 22) as usize])),
            25..=27 => Ok(Self::AudioActivity(AUDIO[(code - 25) as usize])),
            _ => Err(Error::Configuration),
        }
    }
}

/// Attribution identifies the reporting path, not cryptographic authenticity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObservationSource {
    Browser,
    Adapter {
        id: OpaqueId,
        version: OpaqueId,
        simulated: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ObservationReceipt {
    pub(crate) sequence: i64,
    pub(crate) observation: Observation,
    pub(crate) source: ObservationSource,
    pub(crate) received_at: i64,
    pub(crate) expires_at: i64,
}

impl ObservationReceipt {
    pub fn sequence(&self) -> i64 {
        self.sequence
    }
    pub fn observation(&self) -> Observation {
        self.observation
    }
    pub fn source(&self) -> &ObservationSource {
        &self.source
    }
    pub fn received_at(&self) -> i64 {
        self.received_at
    }
    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }
}
