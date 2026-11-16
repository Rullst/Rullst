#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `Record::validate` refuses each corrupt-storage condition on its own.

use super::*;

/// `n` lower-case hex digits.
fn hex(n: usize) -> String {
    "0123456789abcdef".chars().cycle().take(n).collect()
}

/// A valid stored record: an active, unpublished, ready video at revision 5.
pub(in crate::sqlite) fn record() -> Record {
    Record {
        asset: Asset {
            id: Reference::new("lesson").unwrap(),
            scope: Scope::new("school-a", "rust-course").unwrap(),
            owner: Reference::new("teacher").unwrap(),
            metadata: Metadata::new("Ownership", "Notes").unwrap(),
            video: Some(VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap()),
            lifecycle: Lifecycle::Active,
            processing: Processing::Ready,
            published: false,
            pending: false,
            revision: 5,
            updated_at: 1_800_000_000,
            length_seconds: 12,
            mp4_720p: false,
            failure: None,
        },
        marker: format!("rullst-video-{}", hex(32)),
        create_digest: hex(64),
        pending: None,
        notifications: Vec::new(),
        last_notification: 0,
    }
}

fn pending(kind: Kind) -> Pending {
    Pending {
        kind,
        dispatched: false,
        nonce: None,
        until: 0,
        revision: 0,
    }
}

/// `record()` with a live update lease taken at revision 5.
fn leased() -> Record {
    let mut record = record();
    record.asset.pending = true;
    record.pending = Some(Pending {
        kind: Kind::Update,
        dispatched: true,
        nonce: Some(hex(32)),
        until: 1_800_000_045,
        revision: 5,
    });
    record
}

/// `record()` while its creation is planned: no video yet.
fn creating() -> Record {
    let mut record = record();
    record.asset.lifecycle = Lifecycle::Creating;
    record.asset.video = None;
    record.asset.processing = Processing::AwaitingUpload;
    record.asset.pending = true;
    record.pending = Some(pending(Kind::Create));
    record
}

fn refused(name: &str, record: Record) {
    assert_eq!(record.validate(), Err(Error::Configuration), "{name}");
}

#[test]
fn records_at_every_bound_are_valid() {
    assert_eq!(record().validate(), Ok(()));
    let mut full = record();
    full.asset.length_seconds = 604_800;
    full.notifications = vec![hex(64); 32];
    full.last_notification = 1_800_000_000;
    assert_eq!(full.validate(), Ok(()));
    assert_eq!(leased().validate(), Ok(()));
    assert_eq!(creating().validate(), Ok(()));
    let mut rejected = creating();
    rejected.asset.failure = Some(OperationFailure::Rejected);
    assert_eq!(rejected.validate(), Ok(()));
}

#[test]
fn each_asset_rule_is_enforced_alone() {
    let cases: [(&str, fn(&mut Record)); 17] = [
        ("revision", |r| r.asset.revision = 0),
        ("updated_at", |r| r.asset.updated_at = 0),
        ("length", |r| r.asset.length_seconds = 604_801),
        ("notification count", |r| {
            r.notifications = vec![hex(64); 33]
        }),
        ("notification digest", |r| {
            r.notifications = vec!["g".repeat(64)]
        }),
        ("negative notification time", |r| r.last_notification = -1),
        ("future notification", |r| {
            r.last_notification = 1_800_000_001
        }),
        ("create digest", |r| r.create_digest = hex(63)),
        ("marker length", |r| {
            r.marker = format!("rullst-video-{}", hex(33))
        }),
        ("marker prefix", |r| {
            r.marker = format!("rullst_video-{}", hex(32))
        }),
        ("marker digits", |r| {
            r.marker = format!("rullst-video-{}g", hex(31))
        }),
        ("pending flag", |r| r.asset.pending = true),
        ("creating with a video", |r| {
            *r = creating();
            r.asset.video = record().asset.video;
        }),
        ("active without a video", |r| r.asset.video = None),
        ("deleting without an intent", |r| {
            r.asset.lifecycle = Lifecycle::Deleting
        }),
        ("deleted but not missing", |r| {
            r.asset.lifecycle = Lifecycle::Deleted
        }),
        ("published while processing", |r| {
            r.asset.published = true;
            r.asset.processing = Processing::Processing;
        }),
    ];
    for (name, change) in cases {
        let mut record = record();
        change(&mut record);
        refused(name, record);
    }
}

#[test]
fn each_pending_rule_is_enforced_alone() {
    let cases: [(&str, fn(&mut Pending)); 9] = [
        ("negative revision", |p| p.revision = -1),
        ("revision ahead", |p| p.revision = 6),
        ("negative deadline", |p| {
            p.nonce = None;
            p.until = -1;
            p.revision = 0;
        }),
        ("lease without revision", |p| p.revision = 0),
        ("lease without deadline", |p| p.until = 0),
        ("short nonce", |p| p.nonce = Some(hex(31))),
        ("nonce digits", |p| p.nonce = Some("g".repeat(32))),
        ("create while active", |p| {
            p.kind = Kind::Create;
            p.nonce = None;
        }),
        ("delete while active", |p| {
            p.kind = Kind::Delete;
            p.nonce = None;
        }),
    ];
    for (name, change) in cases {
        let mut record = leased();
        change(record.pending.as_mut().unwrap());
        refused(name, record);
    }
}

#[test]
fn a_failure_needs_a_stopped_intent_of_its_kind() {
    let mut dispatched = creating();
    dispatched.asset.failure = Some(OperationFailure::Rejected);
    dispatched.pending.as_mut().unwrap().dispatched = true;
    refused("rejected after dispatch", dispatched);
    let mut leased = leased();
    leased.asset.failure = Some(OperationFailure::Rejected);
    refused("failure under a live lease", leased);
}
