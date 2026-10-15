#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::uploads::detect_kind;

fn sniffed(chunks: &[&[u8]]) -> Sniffer {
    let mut sniffer = Sniffer::default();
    for chunk in chunks {
        sniffer.update(chunk);
    }
    sniffer
}

fn corpus() -> Vec<Vec<u8>> {
    let mut corpus: Vec<Vec<u8>> = [
        &b""[..],
        b"plain text",
        b"line one\r\nline two\ttabbed",
        b"  \n\t<b>bold</b>",
        b"<svg onload=alert(1)>",
        b"\xef\xbb\xbf<?xml version=\"1.0\"?>",
        "\u{200B}\u{2060}<!-- comment -->".as_bytes(),
        b"</closing>",
        b"<",
        b"   <",
        b"< not markup",
        b"a < b",
        b"<1",
        b"see JaVaScRiPt:alert(1)",
        b"jjavascript:",
        b"javascript",
        b"javascrip t:",
        b"nul\x00byte",
        b"bell\x07",
        b"del\x7f",
        b"\xc2\x85 next line",
        b"caf\xc3\xa9 \xe2\x82\xac \xf0\x9f\x98\x80",
        b"bad \xff byte",
        b"truncated \xe2\x82",
        b"overlong \xc0\x80",
        b"surrogate \xed\xa0\x80",
        b"\x89PNG\r\n\x1a\nrest",
        b"\xff\xd8\xff\xe0",
        b"%PDF-1.7\n",
        b"\x00\x00\x00\x18ftypmp42",
        b"\x00\x00\x00\x18ftyp",
        b"\x1a\x45\xdf\xa3webm",
        b"%PDF",
    ]
    .iter()
    .map(|sample| sample.to_vec())
    .collect();
    let mut long = vec![b' '; 70];
    long.extend_from_slice(b"<script>");
    corpus.push(long);
    let mut late = b"%PDF-".to_vec();
    late.resize(200, b'x');
    corpus.push(late);
    corpus
}

/// The whole-object verdict of `UploadPolicy`.
fn whole(sample: &[u8]) -> Option<UploadKind> {
    detect_kind(sample).ok()
}

#[test]
fn streaming_recognition_matches_the_whole_object_check_in_any_chunking() {
    for sample in corpus() {
        let expected = whole(&sample);
        assert_eq!(sniffed(&[&sample]).kind(), expected, "{sample:?}");
        let bytes = sample.iter().map(std::slice::from_ref).collect::<Vec<_>>();
        assert_eq!(sniffed(&bytes).kind(), expected, "bytes of {sample:?}");
        for split in 0..=sample.len() {
            let (head, tail) = sample.split_at(split);
            assert_eq!(
                sniffed(&[head, tail]).kind(),
                expected,
                "{sample:?} split at {split}"
            );
        }
    }
}

fn policy() -> MultipartPolicy {
    MultipartPolicy::default()
}

#[test]
fn no_content_check_admits_everything() {
    assert_eq!(sniffed(&[b"\xff"]).admits(&policy(), true), Some(true));
    assert_eq!(sniffed(&[]).admits(&policy(), false), Some(true));
}

#[test]
fn signatures_decide_as_soon_as_the_bytes_allow() {
    let pdf = policy().expect_signature(*b"%PDF-");
    assert_eq!(sniffed(&[b"%PD"]).admits(&pdf, false), None);
    assert_eq!(sniffed(&[b"%PD"]).admits(&pdf, true), Some(false));
    assert_eq!(sniffed(&[b"%PX"]).admits(&pdf, false), Some(false));
    assert_eq!(sniffed(&[b"%PDF-"]).admits(&pdf, false), Some(true));
    assert_eq!(sniffed(&[b"%PDF-1.7"]).admits(&pdf, true), Some(true));
    assert_eq!(sniffed(&[]).admits(&pdf, true), Some(false));
    // Any one of several signatures admits.
    let either = pdf.expect_signature(*b"\x89PNG");
    assert_eq!(
        sniffed(&[b"\x89PNG\r\n"]).admits(&either, false),
        Some(true)
    );
    assert_eq!(sniffed(&[b"\x89P"]).admits(&either, false), None);
    assert_eq!(sniffed(&[b"GIF8"]).admits(&either, false), Some(false));
    // A 64-byte signature is checked in full.
    let long = policy().expect_signature(vec![b'a'; 64]);
    assert_eq!(sniffed(&[&[b'a'; 63]]).admits(&long, false), None);
    assert_eq!(
        sniffed(&[&[b'a'; 63], b"b"]).admits(&long, false),
        Some(false)
    );
    assert_eq!(sniffed(&[&[b'a'; 64]]).admits(&long, false), Some(true));
}

#[test]
fn allowed_kinds_decide_after_twelve_bytes_or_at_the_end() {
    let pdf = policy().allowed_kinds([UploadKind::Pdf]);
    assert_eq!(sniffed(&[b"%PDF-1.7\nabc"]).admits(&pdf, false), Some(true));
    assert_eq!(sniffed(&[b"%PDF-1.7\nab"]).admits(&pdf, false), None);
    assert_eq!(sniffed(&[b"%PDF-"]).admits(&pdf, true), Some(true));
    assert_eq!(
        sniffed(&[b"\x89PNG\r\n\x1a\nabcd"]).admits(&pdf, false),
        Some(false)
    );
    assert_eq!(sniffed(&[b"plain text!!"]).admits(&pdf, false), Some(false));
    assert_eq!(sniffed(&[b"%PD"]).admits(&pdf, true), Some(false));

    let text = policy().allowed_kinds([UploadKind::PlainText]);
    assert_eq!(sniffed(&[b"plain text!!"]).admits(&text, false), None);
    assert_eq!(sniffed(&[b"plain text!!"]).admits(&text, true), Some(true));
    assert_eq!(
        sniffed(&[b"<svg onload=1"]).admits(&text, false),
        Some(false)
    );
    assert_eq!(sniffed(&[b"short\x00"]).admits(&text, false), None);
    assert_eq!(sniffed(&[b"short\x00"]).admits(&text, true), Some(false));
    assert_eq!(
        sniffed(&[b"truncated \xe2\x82"]).admits(&text, true),
        Some(false)
    );
    assert_eq!(
        sniffed(&[b"%PDF-1.7\nabc"]).admits(&text, false),
        Some(false)
    );

    // Both checks must admit.
    let both = policy()
        .allowed_kinds([UploadKind::Pdf])
        .expect_signature(*b"%PDF-1.");
    assert_eq!(
        sniffed(&[b"%PDF-1.7\nabc"]).admits(&both, false),
        Some(true)
    );
    assert_eq!(
        sniffed(&[b"%PDF-2.0\nabc"]).admits(&both, false),
        Some(false)
    );
}

#[test]
fn the_head_keeps_at_most_64_bytes() {
    let sniffer = sniffed(&[&[b'x'; 40], &[b'y'; 40]]);
    assert_eq!(sniffer.head.len(), 64);
    assert_eq!(&sniffer.head[40..], &[b'y'; 24]);
}

#[test]
fn the_scheme_match_spans_chunks_and_restarts_on_a_mismatch() {
    let mut scan = TextScan::default();
    scan.update(b"java");
    assert_eq!(scan.scheme_matched, 4);
    scan.update(b"SCRIPT");
    assert_eq!(scan.scheme_matched, 10);
    assert!(!scan.denied);
    scan.update(b":");
    assert!(scan.denied);

    let mut scan = TextScan::default();
    scan.update(b"javaj");
    assert_eq!(scan.scheme_matched, 1);
    scan.update("javascript\u{e9}:".as_bytes());
    assert_eq!(scan.scheme_matched, 0);
    assert!(scan.is_plain_at_end());
}

#[test]
fn a_character_split_across_chunks_is_completed_or_denied() {
    let mut scan = TextScan::default();
    scan.update(b"\xf0\x9f");
    assert_eq!(scan.carry, b"\xf0\x9f");
    assert!(!scan.is_plain_at_end());
    scan.update(b"\x98");
    scan.update(b"\x80!");
    assert!(scan.carry.is_empty());
    assert!(scan.is_plain_at_end());

    let mut scan = TextScan::default();
    scan.update(b"\xe2");
    scan.update(b"\x28\xa1");
    assert!(scan.denied);
    // Nothing is read after a denial.
    scan.update(b"\xe2");
    assert!(scan.carry.is_empty());

    assert_eq!(utf8_width(0xc3), 2);
    assert_eq!(utf8_width(0xdf), 2);
    assert_eq!(utf8_width(0xe0), 3);
    assert_eq!(utf8_width(0xef), 3);
    assert_eq!(utf8_width(0xf0), 4);
}

#[test]
fn the_lead_state_follows_the_first_significant_character() {
    let mut scan = TextScan::default();
    scan.update(" \u{FEFF}\t".as_bytes());
    assert_eq!(scan.lead, Lead::Skipping);
    scan.update(b"<");
    assert_eq!(scan.lead, Lead::AfterLt);
    scan.update(b" ");
    assert_eq!(scan.lead, Lead::Done);
    scan.update(b"<b>");
    assert!(!scan.denied);
}
