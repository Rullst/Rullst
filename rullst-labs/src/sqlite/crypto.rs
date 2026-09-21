use crate::{ContentHash, LabError};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use zeroize::Zeroizing;

const MAX_PLAINTEXT: usize = 262_144;

/// Dedicated 32-byte random job-content key. Never reuse application credentials.
#[derive(Clone)]
pub struct ContentKey(Zeroizing<[u8; 32]>);
impl ContentKey {
    pub fn new(bytes: [u8; 32]) -> Result<Self, LabError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.iter().all(|b| *b == 0) {
            return Err(LabError::Configuration);
        }
        Ok(Self(bytes))
    }
    pub(super) fn binding(&self) -> ContentHash {
        ContentHash::of(self.0.as_ref())
    }
    fn key(&self) -> Result<aead::LessSafeKey, LabError> {
        let key = aead::UnboundKey::new(&aead::AES_256_GCM, self.0.as_ref())
            .map_err(|_| LabError::Configuration)?;
        Ok(aead::LessSafeKey::new(key))
    }
    pub(super) fn seal(&self, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, LabError> {
        if aad.is_empty() || aad.len() > 2048 || plaintext.len() > MAX_PLAINTEXT {
            return Err(LabError::Capacity);
        }
        let mut nonce = [0; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| LabError::Integrity)?;
        let mut body = Zeroizing::new(plaintext.to_vec());
        self.key()?
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(aad),
                &mut *body,
            )
            .map_err(|_| LabError::Integrity)?;
        let mut sealed = Vec::with_capacity(12 + body.len());
        sealed.extend_from_slice(&nonce);
        sealed.extend_from_slice(&body);
        Ok(sealed)
    }
    pub(super) fn open(&self, aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, LabError> {
        if aad.is_empty()
            || aad.len() > 2048
            || sealed.len() < 28
            || sealed.len() > MAX_PLAINTEXT + 28
        {
            return Err(LabError::Integrity);
        }
        let nonce: [u8; 12] = sealed[..12].try_into().map_err(|_| LabError::Integrity)?;
        let mut body = Zeroizing::new(sealed[12..].to_vec());
        let size = self
            .key()?
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(aad),
                &mut body,
            )
            .map_err(|_| LabError::Integrity)?
            .len();
        body.truncate(size);
        Ok(body)
    }
}
impl std::fmt::Debug for ContentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ContentKey([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_content_is_bound_to_key_context_and_exact_bytes() {
        let key = ContentKey::new([7; 32]).unwrap();
        let sealed = key
            .seal(
                b"school/course/job-a",
                b"private source and expected answers",
            )
            .unwrap();
        assert!(!sealed.windows(14).any(|w| w == b"private source"));
        assert_eq!(
            &**key.open(b"school/course/job-a", &sealed).unwrap(),
            b"private source and expected answers"
        );
        assert!(key.open(b"school/course/job-b", &sealed).is_err());
        assert!(
            ContentKey::new([8; 32])
                .unwrap()
                .open(b"school/course/job-a", &sealed)
                .is_err()
        );
        let mut changed = sealed.clone();
        changed[15] ^= 1;
        assert!(key.open(b"school/course/job-a", &changed).is_err());
        assert_ne!(
            key.seal(b"school/course/job-a", b"same").unwrap(),
            key.seal(b"school/course/job-a", b"same").unwrap()
        );
        assert!(ContentKey::new([0; 32]).is_err());
    }
}
