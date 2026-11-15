//! AES-256-GCM recovery records: NIST and Wycheproof vectors, and records
//! written by the `ring` implementation that 13.0.0-alpha.5 shipped.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::tests::storage;
use super::*;
use std::collections::BTreeMap;

/// One AES-256-GCM test vector, in hexadecimal.
struct Vector {
    id: &'static str,
    key: &'static str,
    nonce: &'static str,
    aad: &'static str,
    plaintext: &'static str,
    ciphertext: &'static str,
    tag: &'static str,
    valid: bool,
}

/// NIST CAVS `gcmEncryptExtIV256.rsp` and Wycheproof `aes_gcm_test.json`
/// (AES-256, 96-bit nonces, 128-bit tags): valid encryptions and tags with
/// flipped bits.
const VECTORS: &[Vector] = &[
    Vector {
        id: "NIST gcmEncryptExtIV256 plaintext and AAD",
        key: "92e11dcdaa866f5ce790fd24501f92509aacf4cb8b1339d50c9c1240935dd08b",
        nonce: "ac93a1a6145299bde902f21a",
        aad: "1e0889016f67601c8ebea4943bc23ad6",
        plaintext: "2d71bcfa914e4ac045b2aa60955fad24",
        ciphertext: "8995ae2e6df3dbf96fac7b7137bae67f",
        tag: "eca5aa77d51d4a0a14d9c51e1da474ab",
        valid: true,
    },
    Vector {
        id: "NIST gcmEncryptExtIV256 long plaintext and AAD",
        key: "148579a3cbca86d5520d66c0ec71ca5f7e41ba78e56dc6eebd566fed547fe691",
        nonce: "b08a5ea1927499c6ecbfd4e0",
        aad: "e4f963f015ffbb99ee3349bbaf7e8e8e6c2a71c230a48f9d59860a29091d2747e01a5ca572347e247d25f56ba7ae8e05cde2be3c97931292c02370208ecd097ef692687fecf2f419d3200162a6480a57dad408a0dfeb492e2c5d",
        plaintext: "9d0b15fdf1bd595f91f8b3abc0f7dec927dfd4799935a1795d9ce00c9b879434420fe42c275a7cd7b39d638fb81ca52b49dc41",
        ciphertext: "2097e372950a5e9383c675e89eea1c314f999159f5611344b298cda45e62843716f215f82ee663919c64002a5c198d7878fd3f",
        tag: "adbecdb0d5c2224d804d2886ff9a5760",
        valid: true,
    },
    Vector {
        id: "NIST gcmEncryptExtIV256 plaintext only",
        key: "31bdadd96698c204aa9ce1448ea94ae1fb4a9a0b3c9d773b51bb1822666b8f22",
        nonce: "0d18e06c7c725ac9e362e1ce",
        aad: "",
        plaintext: "2db5168e932556f8089a0622981d017d",
        ciphertext: "fa4362189661d163fcd6a56d8bf0405a",
        tag: "d636ac1bbedd5cc3ee727dc2ab4a9489",
        valid: true,
    },
    Vector {
        id: "NIST gcmEncryptExtIV256 AAD only",
        key: "78dc4e0aaf52d935c3c01eea57428f00ca1fd475f5da86a49c8dd73d68c8e223",
        nonce: "d79cf22d504cc793c3fb6c8a",
        aad: "b96baa8c1c75a671bfb2d08d06be5f36",
        plaintext: "",
        ciphertext: "",
        tag: "3e5d486aa2e30b22e040b85723a06e76",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 91",
        key: "92ace3e348cd821092cd921aa3546374299ab46209691bc28b8752d17f123c20",
        nonce: "00112233445566778899aabb",
        aad: "00000000ffffffff",
        plaintext: "00010203040506070809",
        ciphertext: "e27abdd2d2a53d2f136b",
        tag: "9a4a2579529301bcfb71c78d4060f52c",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 92",
        key: "29d3a44f8723dc640239100c365423a312934ac80239212ac3df3421a2098123",
        nonce: "00112233445566778899aabb",
        aad: "aabbccddeeff",
        plaintext: "",
        ciphertext: "",
        tag: "2a7d77fa526b8250cb296078926b5020",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 93",
        key: "80ba3192c803ce965ea371d5ff073cf0f43b6a2ab576b208426e11409c09b9b0",
        nonce: "4da5bf8dfd5852c1ea12379d",
        aad: "",
        plaintext: "",
        ciphertext: "",
        tag: "4771a7c404a472966cea8f73c8bfe17a",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 94",
        key: "cc56b680552eb75008f5484b4cb803fa5063ebd6eab91f6ab6aef4916a766273",
        nonce: "99e23ec48985bccdeeab60f1",
        aad: "",
        plaintext: "2a",
        ciphertext: "06",
        tag: "633c1e9703ef744ffffb40edf9d14355",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 97",
        key: "59d4eafb4de0cfc7d3db99a8f54b15d7b39f0acc8da69763b019c1699f87674a",
        nonce: "2fcb1b38a99e71b84740ad9b",
        aad: "",
        plaintext: "549b365af913f3b081131ccb6b825588",
        ciphertext: "f58c16690122d75356907fd96b570fca",
        tag: "28752c20153092818faba2a334640d6e",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 98",
        key: "3b2458d8176e1621c0cc24c0c0e24c1e80d72f7ee9149a4b166176629616d011",
        nonce: "45aaa3e5d16d2d42dc03445d",
        aad: "",
        plaintext: "3ff1514b1c503915918f0c0c31094a6e1f",
        ciphertext: "73a6b6f45f6ccc5131e07f2caa1f2e2f56",
        tag: "2d7379ec1db5952d4e95d30c340b1b1d",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 102",
        key: "f32364b1d339d82e4f132d8f4a0ec1ff7e746517fa07ef1a7f422f4e25a48194",
        nonce: "5a86a50a0e8a179c734b996d",
        aad: "ab2ac7c44c60bdf8228c7884adb20184",
        plaintext: "43891bccb522b1e72a6b53cf31c074e9d6c2df8e",
        ciphertext: "43dda832e942e286da314daa99bef5071d9d2c78",
        tag: "c3922583476ced575404ddb85dd8cd44",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 103",
        key: "ff0089ee870a4a39f645b0a5da774f7a5911e9696fc9cad646452c2aa8595a12",
        nonce: "bc2a7757d0ce2d8b1f14ccd9",
        aad: "972ab4e06390caae8f99dd6e2187be6c7ff2c08a24be16ef",
        plaintext: "748b28031621d95ee61812b4b4f47d04c6fc2ff3",
        ciphertext: "a929ee7e67c7a2f91bbcec6389a3caf43ab49305",
        tag: "ebec6774b955e789591c822dab739e12",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 117",
        key: "2ce6b4c15f85fb2da5cc6c269491eef281980309181249ebf2832bd6d0732d0b",
        nonce: "c064fae9173b173fd6f11f34",
        aad: "498d3075b09fed998280583d61bb36b6ce41f130063b80824d1586e143d349b126b16aa10fe57343ed223d6364ee602257fe313a7fc9bf9088f027795b8dc1d3",
        plaintext: "f8a27a4baf00dc0555d222f2fa4fb42dc666ea3c",
        ciphertext: "aed58d8a252f740dba4bf6d36773bd5b41234bba",
        tag: "01f93d7456aa184ebb49bea472b6d65d",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 128",
        key: "00112233445566778899aabbccddeeff102132435465768798a9bacbdcedfe0f",
        nonce: "000000000000000000000000",
        aad: "",
        plaintext: "561008fa07a68f5c61285cd013464eaf",
        ciphertext: "23293e9b07ca7d1b0cae7cc489a973b3",
        tag: "ffffffffffffffffffffffffffffffff",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 129",
        key: "00112233445566778899aabbccddeeff102132435465768798a9bacbdcedfe0f",
        nonce: "ffffffffffffffffffffffff",
        aad: "",
        plaintext: "c6152244cea1978d3e0bc274cf8c0b3b",
        ciphertext: "7cb6fc7c6abc009efe9551a99f36a421",
        tag: "00000000000000000000000000000000",
        valid: true,
    },
    Vector {
        id: "Wycheproof tcId 130",
        key: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        nonce: "505152535455565758595a5b",
        aad: "",
        plaintext: "202122232425262728292a2b2c2d2e2f",
        ciphertext: "b2061457c0759fc1749f174ee1ccadfa",
        tag: "9de8fef6d8ab1bf1bf887232eab590dd",
        valid: false,
    },
    Vector {
        id: "Wycheproof tcId 148",
        key: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        nonce: "505152535455565758595a5b",
        aad: "",
        plaintext: "202122232425262728292a2b2c2d2e2f",
        ciphertext: "b2061457c0759fc1749f174ee1ccadfa",
        tag: "9ce8fef6d8ab1bf1bf887232eab5905d",
        valid: false,
    },
    Vector {
        id: "Wycheproof tcId 152",
        key: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        nonce: "505152535455565758595a5b",
        aad: "",
        plaintext: "202122232425262728292a2b2c2d2e2f",
        ciphertext: "b2061457c0759fc1749f174ee1ccadfa",
        tag: "631701092754e40e40778dcd154a6f22",
        valid: false,
    },
    Vector {
        id: "Wycheproof tcId 153",
        key: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        nonce: "505152535455565758595a5b",
        aad: "",
        plaintext: "202122232425262728292a2b2c2d2e2f",
        ciphertext: "b2061457c0759fc1749f174ee1ccadfa",
        tag: "00000000000000000000000000000000",
        valid: false,
    },
    Vector {
        id: "Wycheproof tcId 156",
        key: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        nonce: "505152535455565758595a5b",
        aad: "",
        plaintext: "202122232425262728292a2b2c2d2e2f",
        ciphertext: "b2061457c0759fc1749f174ee1ccadfa",
        tag: "9de9fff7d9aa1af0be897333ebb491dc",
        valid: false,
    },
];

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

/// Records that the `ring` implementation of 13.0.0-alpha.5 wrote for the
/// states of [`fixture_states`] in the storage of [`fixture_storage`]: with a
/// random nonce, and with the nonce `f0 f1 .. fb` of [`FIXED_NONCE`].
const RING_PLAIN: &str = "mp1.9S27lVqExLBPKbizeBPj8Ed0c2n3_7GXnL36BB4sCN94FH8MWeJT1p9SnwkD6LioXb6uY0xQBzmcQ-6vsB7eLifEFqu6KnKvr6XTqxqCv0PdXQJqleE18Af8ZHoiPveipcdRkQKcLqrPJ4of7HbMUGCiXOaIs6qPH49iCx_TG4WA_QH5daxbXrc-JkJEuQywE4OVLw";
const RING_WITH_RECEIPT: &str = "mp1.xY9cugiD6GUxmXs4jxCsZ1dcS4l38BN8lg4q2S-6zI71Nkj2Tk1_1Gl7CW6egJ76BCsTbZNtQPQPTiElLD896QISWxs5tmHzXRytxpEZw_tez9_mR8MVB-8aAAsY1TQnU2hR1BmwTI22ntsvaL_q0IltPCQdU32c8Ble3ATzBympRbg5DZ2RQT7MEd6Sq95qQRY0M_s7lLfLCDzVZ9vu_ZzjanJzn2uj4YJRgAdUyj6m0zUkWZ-MZ-3EP8-qEzAzZrQDAGWZngsBgHAvTHaC5cRmPCIKslKrI4WnmXAjqVaY3Q_ECx39ifFy5i7VJWOn_iK71m3LaLeSOgd4xiVzmA";
const RING_FIXED_PLAIN: &str = "mp1.8PHy8_T19vf4-fr7SVROZn45wnZctQp0eZQlvBERZW53I14cvuQk3ZmyP_l0PmWIepAQYLD4o5DHAVZ0YPwbTqMsF-i3egfhIRiNZijPXGlAdRE6_KQ6vKSTrFV-AAnHEpST0G8ov3nsRp1B9JuZQGX9rsx2tVr4HrEynKDsl4e0Uw_qVKI_fTVBTl9x-4UmT_2wBw";
const RING_FIXED_WITH_RECEIPT: &str = "mp1.8PHy8_T19vf4-fr7SVROZn45wnZctQp0eZQlvBERZW53I14cvuQk3ZmyP_l0PmWIepAQYLD4o5DHAVZ0YPwbTqMsF-i3egfhIRiNZijPXGlAdRE6_KQ6vKSTrFV-AAnHEpST0G8ov3nsRp1B9JuZQGX9rsx2tVr4HrEynKDsl4e0U1CmxuoqMMGT_pcVpDVPixW1GrLPQMgVQXFrvHNCSPgs16USbkwrl3EY2MCeUTmOebG0CHoeRveHvTqZht0SQXHgKkYIyqBMjohLUC_ju1yUhAXtx2aSk-uCikmRt2IDqsX3SYhtJAh4kn9vhQoEYm5jN_VX8SLv2VWANgfFug";

const FIXED_NONCE: [u8; 12] = [
    0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb,
];

/// The storage of `codec::tests` (key `[17; 32]`) bound to `0, 1, .., 31`.
fn fixture_storage() -> MultipartStorage {
    let mut storage = storage();
    storage.binding = std::array::from_fn(|index| index as u8);
    storage
}

/// A record without receipts and one with the receipt of its only part.
fn fixture_states() -> (State, State) {
    let plain = State {
        upload: "fixture-upload".into(),
        marker: "fixture-marker".into(),
        total: 10,
        created: 1_700_000_000,
        expires: 1_700_000_060,
        receipts: BTreeMap::new(),
    };
    let mut with_receipt = plain.clone();
    with_receipt.receipts.insert(
        1,
        super::super::Receipt {
            etag: "\"etag\"".into(),
            size: 10,
            sha256: [7; 32],
        },
    );
    (plain, with_receipt)
}

fn checkpoint(token: &str) -> MultipartCheckpoint {
    MultipartCheckpoint::from_encoded(token).unwrap()
}

#[test]
fn records_written_with_ring_still_open() {
    let storage = fixture_storage();
    for (token, receipts) in [
        (RING_PLAIN, 0),
        (RING_FIXED_PLAIN, 0),
        (RING_WITH_RECEIPT, 1),
        (RING_FIXED_WITH_RECEIPT, 1),
    ] {
        // They expired in 2023, so only cleanup may open them.
        let state = storage.open(&checkpoint(token), true).unwrap();
        assert_eq!(state.upload, "fixture-upload");
        assert_eq!(state.marker, "fixture-marker");
        assert_eq!(
            (state.total, state.created, state.expires),
            (10, 1_700_000_000, 1_700_000_060)
        );
        assert_eq!(state.receipts.len(), receipts);
        if let Some(receipt) = state.receipts.get(&1) {
            assert_eq!(
                (receipt.etag.as_str(), receipt.size, receipt.sha256),
                ("\"etag\"", 10, [7; 32])
            );
        }
        assert!(matches!(
            storage.open(&checkpoint(token), false),
            Err(MultipartError::Expired)
        ));
    }
}

#[test]
fn the_same_nonce_seals_the_bytes_ring_wrote() {
    let storage = fixture_storage();
    let (plain, with_receipt) = fixture_states();
    let sealed = storage.seal_with_nonce(&plain, FIXED_NONCE).unwrap();
    assert_eq!(sealed.expose_encoded(), RING_FIXED_PLAIN);
    let sealed = storage.seal_with_nonce(&with_receipt, FIXED_NONCE).unwrap();
    assert_eq!(sealed.expose_encoded(), RING_FIXED_WITH_RECEIPT);
    // Without a chosen nonce every record gets a fresh one.
    let first = storage.seal(&plain).unwrap();
    let second = storage.seal(&plain).unwrap();
    assert_ne!(first.expose_encoded(), second.expose_encoded());
    for token in [first, second] {
        assert_eq!(storage.open(&token, true).unwrap().upload, "fixture-upload");
    }
}

#[test]
fn ring_records_need_their_key_binding_and_bytes() {
    let storage = fixture_storage();
    let mut other_binding = fixture_storage();
    other_binding.binding[31] ^= 1;
    let mut other_key = fixture_storage();
    other_key.key = MultipartKey::new(URL_SAFE_NO_PAD.encode([18; 32])).unwrap();
    for wrong in [&other_binding, &other_key] {
        assert!(matches!(
            wrong.open(&checkpoint(RING_PLAIN), true),
            Err(MultipartError::InvalidCheckpoint)
        ));
    }
    let record = URL_SAFE_NO_PAD
        .decode(RING_PLAIN.strip_prefix("mp1.").unwrap())
        .unwrap();
    // A flipped bit in the nonce, the ciphertext or the tag.
    for index in [
        0,
        11,
        12,
        record.len() - 17,
        record.len() - 16,
        record.len() - 1,
    ] {
        let mut tampered = record.clone();
        tampered[index] ^= 0x80;
        let token = format!("mp1.{}", URL_SAFE_NO_PAD.encode(tampered));
        assert!(
            matches!(
                storage.open(&checkpoint(&token), true),
                Err(MultipartError::InvalidCheckpoint)
            ),
            "byte {index}"
        );
    }
}

#[test]
fn records_follow_the_nist_and_wycheproof_vectors() {
    for vector in VECTORS {
        let key = MultipartKey::new(URL_SAFE_NO_PAD.encode(hex(vector.key))).unwrap();
        let nonce: [u8; 12] = hex(vector.nonce).try_into().unwrap();
        let aad = hex(vector.aad);
        let record = [hex(vector.nonce), hex(vector.ciphertext), hex(vector.tag)].concat();
        if vector.valid {
            assert_eq!(
                key.open(&aad, &record),
                Ok(hex(vector.plaintext)),
                "{}",
                vector.id
            );
            let sealed = key.seal(nonce, &aad, &hex(vector.plaintext));
            assert_eq!(sealed, Ok(record), "{}", vector.id);
        } else {
            assert_eq!(
                key.open(&aad, &record),
                Err(MultipartError::InvalidCheckpoint),
                "{}",
                vector.id
            );
        }
    }
    let key = MultipartKey::new(URL_SAFE_NO_PAD.encode([17; 32])).unwrap();
    for short in [&[0; 11][..], &[0; 12], &[0; 27]] {
        assert_eq!(key.open(&[], short), Err(MultipartError::InvalidCheckpoint));
    }
}

#[test]
fn keys_are_cleared_on_drop_and_keep_their_auto_traits() {
    fn cleared_on_drop<T: zeroize::ZeroizeOnDrop>() {}
    cleared_on_drop::<Aes256Gcm>();
    fn auto_traits<T: Send + Sync + Unpin + std::panic::RefUnwindSafe + std::panic::UnwindSafe>() {}
    auto_traits::<MultipartKey>();
}
