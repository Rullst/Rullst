#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

/// Every event of `body` pushed in chunks of `chunk` bytes, `NeedMore`
/// excluded, with adjacent `Data` merged.
fn events(body: &[u8], chunk: usize) -> Result<Vec<Event>, MultipartError> {
    let mut parser = Parser::new("XyZ");
    let mut chunks = body.chunks(chunk);
    let mut events = Vec::new();
    loop {
        match parser.next_event()? {
            Event::NeedMore => match chunks.next() {
                Some(chunk) => parser.push(chunk),
                None => parser.finish(),
            },
            Event::End => {
                events.push(Event::End);
                return Ok(events);
            }
            Event::Data(data) => match events.last_mut() {
                Some(Event::Data(previous)) => previous.extend_from_slice(&data),
                _ => events.push(Event::Data(data)),
            },
            event => events.push(event),
        }
    }
}

/// `Data` followed by `PartEnd` folded into one `PartEnd`.
fn folded(events: Vec<Event>) -> Vec<Event> {
    let mut output: Vec<Event> = Vec::new();
    for event in events {
        match (output.last_mut(), event) {
            (Some(Event::Data(data)), Event::PartEnd(rest)) => {
                let mut whole = std::mem::take(data);
                whole.extend_from_slice(&rest);
                output.pop();
                output.push(Event::PartEnd(whole));
            }
            (_, event) => output.push(event),
        }
    }
    output
}

const FORM: &[u8] = b"--XyZ\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nHello\r\n--XyZ\r\nContent-Disposition: form-data; name=\"doc\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\nline one\r\nline two\r\n--XyZ--\r\nepilogue";

fn expected_form() -> Vec<Event> {
    vec![
        Event::PartStart(b"Content-Disposition: form-data; name=\"title\"".to_vec()),
        Event::PartEnd(b"Hello".to_vec()),
        Event::PartStart(
            b"Content-Disposition: form-data; name=\"doc\"; filename=\"a.txt\"\r\nContent-Type: text/plain"
                .to_vec(),
        ),
        Event::PartEnd(b"line one\r\nline two".to_vec()),
        Event::End,
    ]
}

#[test]
fn a_form_parses_the_same_in_any_chunking() {
    for chunk in [1, 2, 3, 5, 7, 64, FORM.len()] {
        assert_eq!(
            folded(events(FORM, chunk).unwrap()),
            expected_form(),
            "{chunk}"
        );
    }
}

#[test]
fn the_end_is_final_and_the_epilogue_is_never_read() {
    let mut parser = Parser::new("XyZ");
    parser.push(b"--XyZ--");
    assert_eq!(parser.next_event().unwrap(), Event::End);
    assert_eq!(parser.consumed(), 7);
    parser.push(b"anything");
    assert_eq!(parser.next_event().unwrap(), Event::End);
    assert_eq!(parser.consumed(), 7);
}

#[test]
fn consumed_counts_bytes_through_each_closing_delimiter() {
    let mut parser = Parser::new("XyZ");
    parser.push(FORM);
    let mut ends = Vec::new();
    loop {
        match parser.next_event().unwrap() {
            Event::PartEnd(_) => ends.push(parser.consumed()),
            Event::End => break,
            _ => {}
        }
    }
    // The first part ends after `Hello\r\n--XyZ` (67 bytes), the second
    // after its delimiter (184 bytes); only `--` follows, never the epilogue.
    assert_eq!(ends, vec![67, 184]);
    assert_eq!(parser.consumed(), 186);
}

#[test]
fn malformed_bodies_fail_with_a_fixed_reason() {
    for (body, reason) in [
        (
            &b"preamble\r\n--XyZ\r\n"[..],
            "the body does not start with the boundary",
        ),
        (b"--XyA", "the body does not start with the boundary"),
        (b"--XyZ  \r\n", "a boundary is followed by unexpected bytes"),
        (
            b"--XyZ\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nvalue\r\n--XyZXY",
            "a boundary is followed by unexpected bytes",
        ),
        (b"", "the body ended before the closing boundary"),
        (b"--Xy", "the body ended before the closing boundary"),
        (b"--XyZ", "the body ended before the closing boundary"),
        (
            b"--XyZ\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nvalue",
            "the body ended before the closing boundary",
        ),
        (
            b"--XyZ\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nvalue\r\n--XyZ",
            "the body ended before the closing boundary",
        ),
    ] {
        for chunk in [1, 4, body.len().max(1)] {
            assert_eq!(
                events(body, chunk),
                Err(MultipartError::Malformed(reason)),
                "{}",
                String::from_utf8_lossy(body)
            );
        }
    }
}

#[test]
fn content_resembling_a_boundary_stays_content() {
    // Another boundary, a delimiter without its CRLF and a lone CR all
    // belong to the part.
    let body = b"--XyZ\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n--XyZ\r\n--inner\r\n\r--XyZ\r\r\n-\r\n--XyZ--";
    for chunk in [1, 3, body.len()] {
        assert_eq!(
            folded(events(body, chunk).unwrap()),
            vec![
                Event::PartStart(b"Content-Disposition: form-data; name=\"a\"".to_vec()),
                Event::PartEnd(b"--XyZ\r\n--inner\r\n\r--XyZ\r\r\n-".to_vec()),
                Event::End,
            ]
        );
    }
}

#[test]
fn a_part_without_headers_starts_with_an_empty_block() {
    let body = b"--XyZ\r\n\r\nvalue\r\n--XyZ--";
    for chunk in [1, body.len()] {
        assert_eq!(
            folded(events(body, chunk).unwrap()),
            vec![
                Event::PartStart(Vec::new()),
                Event::PartEnd(b"value".to_vec()),
                Event::End
            ]
        );
    }
}

#[test]
fn an_empty_value_ends_at_the_next_delimiter() {
    let body = b"--XyZ\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n\r\n--XyZ--";
    for chunk in [1, body.len()] {
        assert_eq!(
            folded(events(body, chunk).unwrap()),
            vec![
                Event::PartStart(b"Content-Disposition: form-data; name=\"a\"".to_vec()),
                Event::PartEnd(Vec::new()),
                Event::End
            ]
        );
    }
}

fn header_block(length: usize) -> Vec<u8> {
    let prefix = b"Content-Disposition: form-data; name=\"a\"\r\nX-Pad: ";
    let mut block = prefix.to_vec();
    block.resize(length, b'p');
    block
}

fn with_headers(block: &[u8]) -> Vec<u8> {
    let mut body = b"--XyZ\r\n".to_vec();
    body.extend_from_slice(block);
    body.extend_from_slice(b"\r\n\r\nv\r\n--XyZ--");
    body
}

#[test]
fn part_headers_are_limited_to_8_kib() {
    for chunk in [1, 1000, 9000] {
        let fits = events(&with_headers(&header_block(8192)), chunk).unwrap();
        assert_eq!(fits[0], Event::PartStart(header_block(8192)), "{chunk}");
        assert_eq!(
            events(&with_headers(&header_block(8193)), chunk),
            Err(MultipartError::PartHeadersTooLarge),
            "{chunk}"
        );
    }
    // An unterminated block fails once it cannot fit, without the body end.
    let mut parser = Parser::new("XyZ");
    let mut body = b"--XyZ\r\n".to_vec();
    body.extend_from_slice(&header_block(8195));
    parser.push(&body);
    assert_eq!(parser.next_event(), Ok(Event::NeedMore));
    parser.push(b"p");
    assert_eq!(
        parser.next_event(),
        Err(MultipartError::PartHeadersTooLarge)
    );
}

#[test]
fn content_is_released_in_bounded_pieces_without_its_delimiter_tail() {
    let mut parser = Parser::new("XyZ");
    parser.push(b"--XyZ\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n");
    assert!(matches!(parser.next_event(), Ok(Event::PartStart(_))));
    // Below 16 KiB of settled content nothing is released.
    parser.push(&vec![b'a'; 16 * 1024]);
    assert_eq!(parser.next_event(), Ok(Event::NeedMore));
    // The last 6 bytes may start the 7-byte delimiter `\r\n--XyZ`.
    parser.push(&[b'b'; 6]);
    let Ok(Event::Data(data)) = parser.next_event() else {
        panic!("settled content is released");
    };
    assert_eq!(data.len(), 16 * 1024);
    assert_eq!(parser.next_event(), Ok(Event::NeedMore));
    parser.push(b"\r\n--XyZ--");
    assert_eq!(parser.next_event(), Ok(Event::PartEnd(vec![b'b'; 6])));
    assert_eq!(parser.next_event(), Ok(Event::End));
}

#[test]
fn many_tiny_parts_in_one_chunk_parse_in_linear_time() {
    let mut body = Vec::new();
    for index in 0..20_000 {
        body.extend_from_slice(
            format!(
                "--XyZ\r\nContent-Disposition: form-data; name=\"f{index}\"\r\n\r\n{index}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(b"--XyZ--");
    let started = std::time::Instant::now();
    let parsed = events(&body, body.len()).unwrap();
    assert_eq!(parsed.len(), 40_001);
    assert_eq!(parsed[39_999], Event::PartEnd(b"19999".to_vec()));
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn consumed_bytes_are_dropped_once_they_are_half_the_buffer() {
    let mut parser = Parser::new("XyZ");
    parser.buffer = vec![b'a'; 10];
    parser.start = 4;
    parser.push(b"b");
    assert_eq!((parser.start, parser.buffer.len()), (4, 11));
    parser.start = 5;
    parser.buffer.truncate(10);
    parser.push(b"b");
    assert_eq!((parser.start, parser.buffer.len()), (0, 6));
    assert_eq!(parser.pending(), b"aaaaab");
}

#[test]
fn find_returns_the_first_complete_match() {
    assert_eq!(find(b"ab\r\n--XyZ", b"\r\n--XyZ"), Some(2));
    assert_eq!(find(b"\r\r\n--XyZ\r\n--XyZ", b"\r\n--XyZ"), Some(1));
    assert_eq!(find(b"\r\n--Xy", b"\r\n--XyZ"), None);
    assert_eq!(find(b"\r\n--XyA\r\n--XyZ", b"\r\n--XyZ"), Some(7));
    assert_eq!(find(b"", b"\r\n"), None);
    assert_eq!(find(b"abc", b""), None);
    assert_eq!(find(b"\r\n", b"\r\n"), Some(0));
}
