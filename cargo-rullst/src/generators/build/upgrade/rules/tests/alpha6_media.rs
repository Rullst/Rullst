//! 13.0.0-alpha.6: `rullst-media`'s `Processing` became `#[non_exhaustive]`
//! and gained `Playable`, so a `match` on it needs a wildcard arm.

use super::*;

const PROCESSING: &str = "V13-MEDIA-PROCESSING-MATCH";

const EXHAUSTIVE: &str = r#"use rullst_media::{Processing, RemoteVideo};

fn label(video: &RemoteVideo) -> &'static str {
    match video.processing {
        Processing::AwaitingUpload => "waiting",
        Processing::Processing => "processing",
        Processing::Ready => "ready",
        Processing::Failed | Processing::Missing => "unavailable",
    }
}
"#;

#[test]
fn exhaustive_processing_matches_must_change_at_every_location() {
    assert_eq!(
        scan_with(EXHAUSTIVE, "lib.rs", &WorkspaceFacts::default()),
        [(PROCESSING, 4)]
    );
    // A fully qualified path, an alias and a second match in the same file.
    let source = "use rullst_media::Processing as State;\nfn a(s: rullst_media::Processing) -> u8 {\n    match s {\n        rullst_media::Processing::AwaitingUpload => 0,\n        rullst_media::Processing::Ready => 1,\n        rullst_media::Processing::Processing | rullst_media::Processing::Failed | rullst_media::Processing::Missing => 2,\n    }\n}\nfn b(s: State) -> bool {\n    match s {\n        State::AwaitingUpload | State::Processing => false,\n        State::Ready | State::Failed | State::Missing => true,\n    }\n}\n";
    assert_eq!(
        scan_with(source, "lib.rs", &WorkspaceFacts::default()),
        [(PROCESSING, 3), (PROCESSING, 10)]
    );
}

#[test]
fn wildcards_comparisons_and_other_enums_are_not_reported() {
    // With a wildcard or a binding, `==`, `matches!` and the new helpers.
    assert_only(
        "use rullst_media::Processing;\nfn a(s: Processing) -> bool { match s { Processing::AwaitingUpload => false, _ => true } }\nfn b(s: Processing) -> bool { match s { Processing::AwaitingUpload => false, other => other.is_playable() } }\nfn c(s: Processing) -> bool { s == Processing::Ready || matches!(s, Processing::AwaitingUpload) }\n",
        &[],
    );
    // An application's own `AwaitingUpload` state, in a file without `rullst_media`.
    assert_only(
        "enum Upload { AwaitingUpload, Done }\nfn a(u: Upload) -> bool { match u { Upload::AwaitingUpload => false, Upload::Done => true } }\n",
        &[],
    );
    // A match on another enum of a file that names the crate: `Action` stays
    // exhaustive, while `PlaybackKind` is reported by the non-exhaustive rule.
    assert_only(
        "use rullst_media::Action;\nfn a(k: Action) -> u8 { match k { Action::Manage => 0, Action::Play => 1 } }\n",
        &[],
    );
    assert_only(
        "use rullst_media::PlaybackKind;\nfn a(k: PlaybackKind) -> u8 { match k { PlaybackKind::Embed => 0, PlaybackKind::Hls => 1, PlaybackKind::Mp4_720p | PlaybackKind::Original => 2 } }\n",
        &["V13-NON-EXHAUSTIVE-MATCH"],
    );
}
