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
    /// Brightness statistics of one camera frame, computed where it was
    /// captured, with no image (v13). Built with
    /// [`MediaSample::luminance`], never from bytes.
    LuminanceStatistics,
}

/// Brightness statistics of one camera frame on the 0–255 luma scale,
/// computed by the capturing client so that no image leaves it (v13). A
/// privacy-preserving camera-presence adapter can classify a covered or dark
/// camera from these alone. Values are checked for range, not authenticity:
/// like any client report, they can be forged.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LuminanceStatistics {
    mean: f32,
    standard_deviation: f32,
    samples: u32,
}
impl LuminanceStatistics {
    /// Largest number of luminance values one summary may describe
    /// (a 4096×4096 frame).
    pub const MAX_SAMPLES: u32 = 16_777_216;

    /// `mean` within 0–255, the population `standard_deviation` within
    /// 0–127.5 (the largest possible on that scale) and the number of
    /// luminance values summarized (`samples`, 1 to [`Self::MAX_SAMPLES`]),
    /// for example 192 for a 16×12 grid of block means.
    pub fn new(mean: f32, standard_deviation: f32, samples: u32) -> Result<Self, Error> {
        if !mean.is_finite()
            || !standard_deviation.is_finite()
            || !(0.0..=255.0).contains(&mean)
            || !(0.0..=127.5).contains(&standard_deviation)
            || !(1..=Self::MAX_SAMPLES).contains(&samples)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            mean,
            standard_deviation,
            samples,
        })
    }
    pub fn mean(self) -> f32 {
        self.mean
    }
    pub fn standard_deviation(self) -> f32 {
        self.standard_deviation
    }
    pub fn samples(self) -> u32 {
        self.samples
    }
}

/// Borrowed caller-owned bytes, or brightness statistics without any image.
/// The host binds capture to a participant's current permission and session.
/// Bounds/signature checks do not decode or attest media.
#[derive(Clone, Copy)]
pub struct MediaSample<'a> {
    format: SampleFormat,
    bytes: &'a [u8],
    statistics: Option<LuminanceStatistics>,
}
impl MediaSample<'static> {
    /// A camera-presence sample that carries only brightness statistics
    /// (`SampleFormat::LuminanceStatistics`, v13); `bytes()` is empty.
    pub fn luminance(statistics: LuminanceStatistics) -> Self {
        Self {
            format: SampleFormat::LuminanceStatistics,
            bytes: &[],
            statistics: Some(statistics),
        }
    }
}
impl<'a> MediaSample<'a> {
    /// Frames and audio from bytes. `SampleFormat::LuminanceStatistics` is
    /// refused here (`InvalidInput`): use [`MediaSample::luminance`].
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
            SampleFormat::LuminanceStatistics => false,
        };
        if !valid {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            format,
            bytes,
            statistics: None,
        })
    }
    pub fn format(self) -> SampleFormat {
        self.format
    }
    /// The frame or audio bytes; empty for a statistics sample.
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    /// The brightness statistics of a `LuminanceStatistics` sample (v13).
    pub fn statistics(self) -> Option<LuminanceStatistics> {
        self.statistics
    }
    pub fn kind(self) -> AnalysisKind {
        match self.format {
            SampleFormat::JpegFrame
            | SampleFormat::PngFrame
            | SampleFormat::LuminanceStatistics => AnalysisKind::CameraPresence,
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
