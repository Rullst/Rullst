//! Private shared verification engine. Public entry points enforce ceremony state.
use super::*;

impl PasskeyAuth {
    pub(in crate::auth::passkey) fn parse_client_data(
        &self,
        encoded: &str,
        expected_type: &str,
    ) -> Result<(Vec<u8>, CollectedClientData), AuthError> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|error| passkey_error(format!("failed to decode clientDataJSON: {error}")))?;
        let client_data: CollectedClientData = serde_json::from_slice(&bytes)
            .map_err(|error| passkey_error(format!("failed to parse clientDataJSON: {error}")))?;

        if client_data.ceremony_type != expected_type {
            return Err(passkey_error("clientDataJSON ceremony type mismatch"));
        }
        if client_data.origin != self.rp_origin {
            return Err(passkey_error("Origin mismatch"));
        }
        if client_data.cross_origin {
            return Err(passkey_error(
                "cross-origin WebAuthn ceremony is not allowed",
            ));
        }
        Ok((bytes, client_data))
    }

    pub(in crate::auth::passkey) fn validate_authenticator_data(
        &self,
        auth_data: &[u8],
    ) -> Result<u32, AuthError> {
        if auth_data.len() < 37 {
            return Err(passkey_error("authenticatorData too short"));
        }
        let expected_hash = sha2::Sha256::digest(self.rp_id.as_bytes());
        if auth_data[..32].ct_eq(expected_hash.as_slice()).unwrap_u8() != 1 {
            return Err(passkey_error("rpIdHash mismatch in authenticatorData"));
        }

        let flags = auth_data[32];
        if flags & FLAG_USER_PRESENT == 0 {
            return Err(passkey_error("authenticator did not prove user presence"));
        }
        if self.require_user_verification && flags & FLAG_USER_VERIFIED == 0 {
            return Err(passkey_error(
                "authenticator did not prove user verification",
            ));
        }
        if flags & FLAG_BACKUP_STATE != 0 && flags & FLAG_BACKUP_ELIGIBLE == 0 {
            return Err(passkey_error("invalid authenticator backup flags"));
        }

        Ok(u32::from_be_bytes([
            auth_data[33],
            auth_data[34],
            auth_data[35],
            auth_data[36],
        ]))
    }

    pub(in crate::auth::passkey) fn decode_credential_id(
        id: &str,
        raw_id: &str,
        credential_type: &str,
    ) -> Result<Vec<u8>, AuthError> {
        if credential_type != "public-key" {
            return Err(passkey_error("credential type must be public-key"));
        }
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(raw_id)
            .map_err(|_| passkey_error("raw credential ID is not valid base64url"))?;
        let encoded_id = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(id)
            .map_err(|_| passkey_error("credential ID is not valid base64url"))?;
        if raw.is_empty() || raw.len() > MAX_CREDENTIAL_ID_BYTES {
            return Err(passkey_error("credential ID length is invalid"));
        }
        if raw.as_slice().ct_eq(&encoded_id).unwrap_u8() != 1 {
            return Err(passkey_error("credential ID and raw ID do not match"));
        }
        Ok(raw)
    }

    pub(in crate::auth::passkey) fn verify_registration(
        &self,
        credential: &RegisterPublicKeyCredential,
        credential_id: &[u8],
    ) -> Result<Passkey, AuthError> {
        let attestation = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.attestation_object)
            .map_err(|error| {
                passkey_error(format!("failed to decode attestationObject: {error}"))
            })?;
        let (attestation, trailing) = parse_cbor(&attestation)?;
        if !trailing.is_empty() {
            return Err(passkey_error("trailing data in attestationObject"));
        }
        let CborValue::Map(mut attestation) = attestation else {
            return Err(passkey_error("attestationObject is not a map"));
        };
        let format = attestation.remove(&CborKey::TextString("fmt".to_owned()));
        if !matches!(format, Some(CborValue::TextString(value)) if value == "none") {
            return Err(passkey_error(
                "only the WebAuthn none attestation format is supported",
            ));
        }
        let statement = attestation.remove(&CborKey::TextString("attStmt".to_owned()));
        if !matches!(statement, Some(CborValue::Map(value)) if value.is_empty()) {
            return Err(passkey_error("none attestation must have an empty attStmt"));
        }
        let Some(CborValue::ByteString(auth_data)) =
            attestation.remove(&CborKey::TextString("authData".to_owned()))
        else {
            return Err(passkey_error("authData not found in attestationObject"));
        };
        if !attestation.is_empty() || auth_data.len() < 55 {
            return Err(passkey_error(
                "invalid attestationObject or authData length",
            ));
        }

        let sign_count = self.validate_authenticator_data(&auth_data)?;
        let flags = auth_data[32];
        if flags & FLAG_ATTESTED_CREDENTIAL_DATA == 0 {
            return Err(passkey_error("attested credential data flag is missing"));
        }
        if flags & FLAG_EXTENSION_DATA != 0 {
            return Err(passkey_error(
                "unrequested authenticator extensions are unsupported",
            ));
        }

        let credential_id_len = u16::from_be_bytes([auth_data[53], auth_data[54]]) as usize;
        if credential_id_len == 0
            || credential_id_len > MAX_CREDENTIAL_ID_BYTES
            || auth_data.len() < 55 + credential_id_len
        {
            return Err(passkey_error("invalid attested credential ID length"));
        }
        let attested_id = &auth_data[55..55 + credential_id_len];
        if attested_id.ct_eq(credential_id).unwrap_u8() != 1 {
            return Err(passkey_error(
                "response credential ID does not match attested credential ID",
            ));
        }

        let (cose_key, trailing) = parse_cbor(&auth_data[55 + credential_id_len..])?;
        if !trailing.is_empty() {
            return Err(passkey_error("trailing data after credential public key"));
        }
        let CborValue::Map(mut cose_key) = cose_key else {
            return Err(passkey_error("credentialPublicKey is not a CBOR map"));
        };
        for (label, expected, name) in [(1, 2, "kty"), (3, -7, "alg"), (-1, 1, "curve")] {
            if !matches!(cose_key.remove(&CborKey::Integer(label)), Some(CborValue::Integer(value)) if value == expected)
            {
                return Err(passkey_error(format!("unsupported or missing COSE {name}")));
            }
        }
        let Some(CborValue::ByteString(x)) = cose_key.remove(&CborKey::Integer(-2)) else {
            return Err(passkey_error("X coordinate missing from public key"));
        };
        let Some(CborValue::ByteString(y)) = cose_key.remove(&CborKey::Integer(-3)) else {
            return Err(passkey_error("Y coordinate missing from public key"));
        };
        if x.len() != 32 || y.len() != 32 {
            return Err(passkey_error(
                "ES256 coordinates must each contain 32 bytes",
            ));
        }
        let mut public_key = Vec::with_capacity(65);
        public_key.push(0x04);
        public_key.extend_from_slice(&x);
        public_key.extend_from_slice(&y);
        p256::PublicKey::from_sec1_bytes(&public_key)
            .map_err(|_| passkey_error("ES256 public key is not a valid P-256 point"))?;

        Ok(Passkey {
            credential_id: credential_id.to_vec(),
            public_key,
            sign_count,
        })
    }

    pub(in crate::auth::passkey) fn verify_assertion(
        &self,
        credential: &PublicKeyCredential,
        client_data_bytes: &[u8],
        mut passkey: Passkey,
    ) -> Result<Passkey, AuthError> {
        let auth_data = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.authenticator_data)
            .map_err(|error| {
                passkey_error(format!("failed to decode authenticatorData: {error}"))
            })?;
        let new_sign_count = self.validate_authenticator_data(&auth_data)?;
        if auth_data[32] & (FLAG_ATTESTED_CREDENTIAL_DATA | FLAG_EXTENSION_DATA) != 0
            || auth_data.len() != 37
        {
            return Err(passkey_error(
                "unexpected data in assertion authenticatorData",
            ));
        }

        let signature_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&credential.response.signature)
            .map_err(|error| passkey_error(format!("failed to decode signature: {error}")))?;
        let client_hash = sha2::Sha256::digest(client_data_bytes);
        let mut signed_message = Vec::with_capacity(auth_data.len() + client_hash.len());
        signed_message.extend_from_slice(&auth_data);
        signed_message.extend_from_slice(&client_hash);
        signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &passkey.public_key)
            .verify(&signed_message, &signature_bytes)
            .map_err(|_| passkey_error("ES256 assertion signature verification failed"))?;

        if (passkey.sign_count != 0 || new_sign_count != 0) && new_sign_count <= passkey.sign_count
        {
            return Err(passkey_error(
                "authenticator signature counter did not advance monotonically",
            ));
        }
        passkey.sign_count = new_sign_count;
        Ok(passkey)
    }
}
