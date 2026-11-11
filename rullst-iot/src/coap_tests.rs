use super::*;
use alloc::vec;

#[test]
fn encodes_well_known_core_get_vector() {
    let datagram = CoapRequest::new(
        CoapMessageType::Confirmable,
        CoapMethod::Get,
        0x7d34,
        Vec::<u8>::new(),
    )
    .and_then(|request| request.path_segment(".well-known"))
    .and_then(|request| request.path_segment("core"))
    .and_then(|request| request.encode())
    .expect("bounded CoAP GET should encode");

    assert_eq!(
        datagram,
        [
            0x40, 0x01, 0x7d, 0x34, 0xbb, b'.', b'w', b'e', b'l', b'l', b'-', b'k', b'n', b'o',
            b'w', b'n', 0x04, b'c', b'o', b'r', b'e',
        ]
    );
}

#[test]
fn encodes_token_content_format_and_payload_marker() {
    let datagram = CoapRequest::new(CoapMessageType::NonConfirmable, CoapMethod::Post, 7, [0xa1])
        .and_then(|request| request.path_segment("telemetry"))
        .and_then(|request| request.content_format(50).payload(b"{}".to_vec()))
        .and_then(|request| request.encode())
        .expect("bounded CoAP POST should encode");
    assert_eq!(
        datagram,
        [
            0x51, 0x02, 0x00, 0x07, 0xa1, 0xb9, b't', b'e', b'l', b'e', b'm', b'e', b't', b'r',
            b'y', 0x11, 0x32, 0xff, b'{', b'}',
        ]
    );
}

#[test]
fn rejects_invalid_or_unbounded_input() {
    assert_eq!(
        CoapRequest::new(CoapMessageType::Confirmable, CoapMethod::Get, 1, [0; 9]),
        Err(CoapCodecError::TokenTooLong)
    );
    assert_eq!(
        CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Get,
            1,
            Vec::<u8>::new()
        )
        .and_then(|request| request.path_segment("bad/path")),
        Err(CoapCodecError::InvalidPathSegment)
    );
    for dot_segment in [".", ".."] {
        assert_eq!(
            CoapRequest::new(
                CoapMessageType::Confirmable,
                CoapMethod::Get,
                1,
                Vec::<u8>::new()
            )
            .and_then(|request| request.path_segment("files"))
            .and_then(|request| request.path_segment(dot_segment)),
            Err(CoapCodecError::InvalidPathSegment)
        );
    }
    assert_eq!(
        CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Put,
            1,
            Vec::<u8>::new()
        )
        .and_then(|request| request.payload(vec![0; MAX_COAP_DATAGRAM_BYTES]))
        .and_then(|request| request.encode()),
        Err(CoapCodecError::DatagramTooLarge)
    );
}

#[test]
fn uri_path_segments_follow_the_rfc_7252_length_limit() {
    let request = || {
        CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Get,
            1,
            Vec::<u8>::new(),
        )
    };
    let datagram = request()
        .and_then(|request| request.path_segment("a".repeat(255)))
        .and_then(|request| request.encode())
        .expect("a 255-byte Uri-Path segment should encode");
    assert_eq!(datagram.len(), 4 + 2 + 255);
    assert_eq!(datagram[4], 0xbd);
    assert_eq!(datagram[5], 255 - 13);

    for length in [256, 300, 1_000] {
        assert_eq!(
            request().and_then(|request| request.path_segment("a".repeat(length))),
            Err(CoapCodecError::OptionTooLarge)
        );
    }
}

#[test]
fn path_bound_uses_the_exact_uri_path_option_size() {
    // (segment length, segments that fit, encoded bytes at that count):
    // one header byte per option, plus one extension byte from 13 bytes.
    for (length, fitting, encoded) in [(1, 574, 1_152), (12, 88, 1_148), (13, 76, 1_144)] {
        let mut request = CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Get,
            1,
            Vec::<u8>::new(),
        )
        .expect("empty token");
        for _ in 0..fitting {
            request = request
                .path_segment("a".repeat(length))
                .expect("segment fits the datagram ceiling");
        }
        assert_eq!(request.encode().map(|datagram| datagram.len()), Ok(encoded));
        assert_eq!(
            request.path_segment("a".repeat(length)),
            Err(CoapCodecError::DatagramTooLarge)
        );
    }
}

#[test]
fn classifies_every_option_encoding_boundary() {
    assert_eq!(
        classify_option_component(12),
        Some(CoapOptionComponent::Inline(12))
    );
    assert_eq!(
        classify_option_component(13),
        Some(CoapOptionComponent::ExtendedOne(0))
    );
    assert_eq!(
        classify_option_component(268),
        Some(CoapOptionComponent::ExtendedOne(u8::MAX))
    );
    assert_eq!(
        classify_option_component(269),
        Some(CoapOptionComponent::ExtendedTwo([0, 0]))
    );
    assert_eq!(
        classify_option_component(65_804),
        Some(CoapOptionComponent::ExtendedTwo([u8::MAX, u8::MAX]))
    );
    assert_eq!(classify_option_component(65_805), None);
}

fn empty_get() -> Result<CoapRequest, CoapCodecError> {
    CoapRequest::new(
        CoapMessageType::Confirmable,
        CoapMethod::Get,
        1,
        Vec::<u8>::new(),
    )
}

#[test]
fn codec_errors_have_stable_messages() {
    use alloc::string::ToString;

    for (error, message) in [
        (
            CoapCodecError::TokenTooLong,
            "CoAP token exceeds eight bytes",
        ),
        (
            CoapCodecError::InvalidPathSegment,
            "CoAP URI path segment is invalid",
        ),
        (
            CoapCodecError::OptionTooLarge,
            "CoAP option cannot be represented",
        ),
        (
            CoapCodecError::DatagramTooLarge,
            "CoAP request exceeds the local datagram ceiling",
        ),
        (
            CoapCodecError::AllocationFailed,
            "CoAP request buffer could not be allocated",
        ),
    ] {
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn token_bytes_count_toward_the_path_bound() {
    // 4 header bytes + a 2-byte token + 573 two-byte options = 1_152.
    let mut request = CoapRequest::new(
        CoapMessageType::Confirmable,
        CoapMethod::Get,
        1,
        [0xa1, 0xa2],
    )
    .expect("short token");
    for _ in 0..573 {
        request = request
            .path_segment("a")
            .expect("segment fits the datagram ceiling");
    }
    assert_eq!(request.encode().map(|datagram| datagram.len()), Ok(1_152));
    assert_eq!(
        request.path_segment("a"),
        Err(CoapCodecError::DatagramTooLarge)
    );
}

#[test]
fn payload_setter_accepts_exactly_the_datagram_ceiling() {
    assert!(
        empty_get()
            .and_then(|request| request.payload(vec![0; MAX_COAP_DATAGRAM_BYTES]))
            .is_ok()
    );
    assert_eq!(
        empty_get().and_then(|request| request.payload(vec![0; MAX_COAP_DATAGRAM_BYTES + 1])),
        Err(CoapCodecError::DatagramTooLarge)
    );
}

#[test]
fn encoded_datagram_is_bounded_by_its_measured_length() {
    // 4 header bytes + 0xff marker + 1_147 payload bytes = 1_152.
    let at_ceiling = empty_get()
        .and_then(|request| request.payload(vec![0; MAX_COAP_DATAGRAM_BYTES - 5]))
        .and_then(|request| request.encode())
        .expect("a datagram at the ceiling should encode");
    assert_eq!(at_ceiling.len(), MAX_COAP_DATAGRAM_BYTES);
    // The payload setter accepts it, but the header and marker push it over.
    assert_eq!(
        empty_get()
            .and_then(|request| request.payload(vec![0; MAX_COAP_DATAGRAM_BYTES - 4]))
            .and_then(|request| request.encode()),
        Err(CoapCodecError::DatagramTooLarge)
    );
    // Options count toward the ceiling too: a 255-byte segment is 257 bytes.
    assert_eq!(
        empty_get()
            .and_then(|request| request.path_segment("a".repeat(255)))
            .and_then(|request| request.payload(vec![0; MAX_COAP_DATAGRAM_BYTES - 261]))
            .and_then(|request| request.encode())
            .map(|datagram| datagram.len()),
        Err(CoapCodecError::DatagramTooLarge)
    );
}

#[test]
fn encoded_datagram_buffer_has_exactly_its_length() {
    let shapes = [
        empty_get(),
        empty_get().and_then(|request| request.path_segment("a".repeat(13))),
        empty_get().and_then(|request| request.path_segment("a".repeat(255))),
        CoapRequest::new(CoapMessageType::Confirmable, CoapMethod::Put, 2, [9; 8])
            .and_then(|request| request.path_segment("fw"))
            .map(|request| request.content_format(11_542))
            .and_then(|request| request.payload(vec![0xab; 300])),
        empty_get().map(|request| request.content_format(0)),
    ];
    for request in shapes {
        let request = request.expect("bounded request");
        let datagram = request.encode().expect("bounded request should encode");
        let mut length = ByteCount(0);
        request.write_to(&mut length).expect("measured");
        assert_eq!(datagram.len(), length.0);
        assert_eq!(datagram.capacity(), datagram.len());
    }
}

#[test]
fn byte_count_saturates_instead_of_wrapping() {
    let mut length = ByteCount(usize::MAX - 1);
    length.put(&[0, 0, 0]);
    assert_eq!(length.0, usize::MAX);
}

#[test]
fn option_extensions_and_uints_take_no_heap() {
    assert_eq!(
        option_component(12).map(|(n, e)| (n, e.as_slice().to_vec())),
        Ok((12, vec![]))
    );
    assert_eq!(
        option_component(13).map(|(n, e)| (n, e.as_slice().to_vec())),
        Ok((13, vec![0]))
    );
    assert_eq!(
        option_component(65_804).map(|(n, e)| (n, e.as_slice().to_vec())),
        Ok((14, vec![0xff, 0xff]))
    );
    assert_eq!(
        option_component(65_805),
        Err(CoapCodecError::OptionTooLarge)
    );
    assert_eq!(minimal_u16(0), ([0, 0], 0));
    assert_eq!(minimal_u16(255), ([255, 0], 1));
    assert_eq!(minimal_u16(256), ([1, 0], 2));
}

#[test]
fn content_format_uses_the_shortest_uint_encoding() {
    for (format, option) in [
        (0, &[0xc0][..]),
        (50, &[0xc1, 0x32][..]),
        (11_542, &[0xc2, 0x2d, 0x16][..]),
    ] {
        let datagram = empty_get()
            .and_then(|request| request.content_format(format).encode())
            .expect("Content-Format option should encode");
        assert_eq!(&datagram[4..], option);
    }
}
