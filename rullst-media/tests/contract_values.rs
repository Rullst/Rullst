//! Exact values of the provider-neutral contracts.
use rullst_media::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn system_clock_reads_the_current_unix_second() {
    let before = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let now = SystemClock.now().unwrap();
    let after = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    assert!(now >= i64::try_from(before.as_secs()).unwrap());
    assert!(now <= i64::try_from(after.as_secs()).unwrap());
    // 2026-11-01T00:00:00Z: the clock is not a small placeholder value.
    assert!(now > 1_793_491_200);
}

#[test]
fn library_id_converts_back_to_its_number() {
    assert_eq!(i64::from(LibraryId::new(7).unwrap()), 7);
    assert_eq!(i64::from(LibraryId::new(424_242).unwrap()), 424_242);
}

#[test]
fn declared_uploads_are_bounded_at_exactly_five_gibibytes() {
    assert_eq!(UploadDeclaration::MAX_LENGTH, 5_368_709_120);
    let largest = UploadDeclaration::new("video/mp4", 5_368_709_120).unwrap();
    assert_eq!(largest.length(), 5_368_709_120);
    assert_eq!(largest.content_type(), "video/mp4");
    assert_eq!(
        UploadDeclaration::new("video/mp4", 5_368_709_121),
        Err(MediaError::InvalidInput)
    );
}
