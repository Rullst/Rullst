//! Opt-in bounded adapter orchestration. No model, capture engine or verdict.
#[cfg(feature = "sqlite")]
use crate::exam::Observation;
use crate::{
    Context, OpaqueId, Scope, SupervisionError as Error,
    exam::{AudioObservation, Capability, PresenceObservation},
};
use std::{fmt, future::Future, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalysisKind {
    CameraPresence,
    AudioActivity,
}
impl AnalysisKind {
    pub const fn capability(self) -> Capability {
        match self {
            Self::CameraPresence => Capability::CameraPresence,
            Self::AudioActivity => Capability::AudioActivity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SampleFormat {
    JpegFrame,
    PngFrame,
    Pcm16LeMono16Khz,
}

/// Borrowed caller-owned bytes. The host binds capture to a participant's current
/// permission and session. Bounds/signature checks do not decode or attest media.
#[derive(Clone, Copy)]
pub struct MediaSample<'a> {
    format: SampleFormat,
    bytes: &'a [u8],
}
impl<'a> MediaSample<'a> {
    pub fn new(format: SampleFormat, bytes: &'a [u8]) -> Result<Self, Error> {
        let valid = match format {
            SampleFormat::JpegFrame => {
                bytes.len() <= 1024 * 1024 && bytes.starts_with(&[255, 216, 255])
            }
            SampleFormat::PngFrame => {
                bytes.len() <= 1024 * 1024 && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            }
            SampleFormat::Pcm16LeMono16Khz => {
                !bytes.is_empty() && bytes.len() <= 160_000 && bytes.len().is_multiple_of(2)
            }
        };
        if !valid {
            return Err(Error::InvalidInput);
        }
        Ok(Self { format, bytes })
    }
    pub fn format(self) -> SampleFormat {
        self.format
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn kind(self) -> AnalysisKind {
        match self.format {
            SampleFormat::JpegFrame | SampleFormat::PngFrame => AnalysisKind::CameraPresence,
            SampleFormat::Pcm16LeMono16Khz => AnalysisKind::AudioActivity,
        }
    }
}
impl fmt::Debug for MediaSample<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MediaSample")
            .field("format", &self.format)
            .field("bytes", &"[withheld]")
            .finish()
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AnalyzerDescriptor {
    id: OpaqueId,
    version: OpaqueId,
    kind: AnalysisKind,
    simulated: bool,
}
impl AnalyzerDescriptor {
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        kind: AnalysisKind,
        simulated: bool,
    ) -> Result<Self, Error> {
        Ok(Self {
            id: OpaqueId::new(id)?,
            version: OpaqueId::new(version)?,
            kind,
            simulated,
        })
    }
    pub fn id(&self) -> &OpaqueId {
        &self.id
    }
    pub fn version(&self) -> &OpaqueId {
        &self.version
    }
    pub fn kind(&self) -> AnalysisKind {
        self.kind
    }
    pub fn simulated(&self) -> bool {
        self.simulated
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Finding {
    Camera(PresenceObservation),
    Audio(AudioObservation),
}
impl Finding {
    pub fn kind(self) -> AnalysisKind {
        match self {
            Self::Camera(_) => AnalysisKind::CameraPresence,
            Self::Audio(_) => AnalysisKind::AudioActivity,
        }
    }
    #[cfg(feature = "sqlite")]
    pub(crate) fn observation(self) -> Observation {
        match self {
            Self::Camera(value) => Observation::CameraPresence(value),
            Self::Audio(value) => Observation::AudioActivity(value),
        }
    }
}

/// Implementations must bound their own decoding/network/CPU/memory, honor
/// cancellation and withhold captured media/credentials from errors and logs.
/// Descriptor metadata is host attribution, not third-party authentication.
pub trait Analyzer: Sync {
    fn descriptor(&self) -> &AnalyzerDescriptor;
    fn analyze(
        &self,
        sample: MediaSample<'_>,
    ) -> impl Future<Output = Result<Finding, Error>> + Send;
}

/// Refresh real application authorization both before analysis and before
/// recording. Shape-valid Context alone cannot establish membership/entitlement.
pub trait AnalysisAuthorization: Sync {
    fn authorize(
        &self,
        context: &Context,
        scope: &Scope,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

#[derive(Debug, Clone, Copy)]
pub struct AnalysisOptions {
    seconds: u32,
    simulated: bool,
}
impl AnalysisOptions {
    pub fn new(timeout_seconds: u32) -> Result<Self, Error> {
        if !(1..=15).contains(&timeout_seconds) {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            seconds: timeout_seconds,
            simulated: false,
        })
    }
    /// Explicit diagnostic mode. Simulated results remain visibly attributed.
    pub fn allow_simulated_for_testing(mut self) -> Self {
        self.simulated = true;
        self
    }
    pub fn timeout(self) -> Duration {
        Duration::from_secs(self.seconds.into())
    }
    pub fn timeout_seconds(self) -> u32 {
        self.seconds
    }
    pub fn permits_simulation(self) -> bool {
        self.simulated
    }
}
