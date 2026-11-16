//! 13.0.0-alpha.6: `rullst-media`'s `Processing` became `#[non_exhaustive]`
//! and gained `Playable`. [`super::rust`] calls the match rule with each
//! `match` that has no catch-all arm.

use super::Rule;
use super::catalog::MEDIA_PROCESSING_MATCH;

/// The rule for a `match` without a catch-all arm, given whether its arm
/// patterns name an identifier and the file's text: arms that name
/// `AwaitingUpload`, a state only `rullst-media`'s `Processing` has, in a file
/// that names `rullst_media` (the facade does not re-export it).
pub(super) fn match_rule(names: impl Fn(&str) -> bool, text: &str) -> Option<&'static Rule> {
    (text.contains("rullst_media") && names("AwaitingUpload")).then_some(&MEDIA_PROCESSING_MATCH)
}
