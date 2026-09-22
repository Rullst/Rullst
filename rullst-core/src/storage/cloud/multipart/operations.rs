use super::super::CloudError;
use super::{
    AbortStatus, CompletionStatus, MultipartCheckpoint, MultipartError, MultipartPart,
    MultipartStorage, Receipt, State, codec, protocol,
};
use reqwest::{
    Method,
    header::{HeaderMap, HeaderValue},
};
use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;

impl MultipartStorage {
    /// Initiates a private upload of exactly this length. Persist the returned checkpoint.
    /// A lost initiation response requires provider lifecycle cleanup.
    pub async fn begin(&self, total_bytes: u64) -> Result<MultipartCheckpoint, MultipartError> {
        if total_bytes == 0 || total_bytes > self.limits.max_total {
            return Err(MultipartError::InvalidInput);
        }
        let created = codec::now()?;
        let marker = uuid::Uuid::new_v4().simple().to_string();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-amz-meta-rullst-upload",
            HeaderValue::from_str(&marker).map_err(|_| MultipartError::InvalidInput)?,
        );
        headers.insert(
            "content-type",
            HeaderValue::from_static("application/octet-stream"),
        );
        let response = self
            .client
            .multipart_request(
                Method::POST,
                &self.object,
                &[("uploads", String::new())],
                headers,
                &[],
            )
            .await?;
        response.expect(200)?;
        let doc = protocol::parse(&response.body, "InitiateMultipartUploadResult")?;
        let root = doc.root_element();
        if protocol::field(root, "Bucket")? != self.client.bucket
            || protocol::field(root, "Key")? != self.object
        {
            return Err(MultipartError::InvalidResponse);
        }
        let upload = protocol::field(root, "UploadId")?;
        if !codec::protocol_token(upload, 2048) {
            return Err(MultipartError::InvalidResponse);
        }
        self.seal(&State {
            upload: upload.into(),
            marker,
            total: total_bytes,
            created,
            expires: created
                .checked_add(self.limits.lifetime.as_secs())
                .ok_or(MultipartError::Expired)?,
            receipts: BTreeMap::new(),
        })
    }

    /// Verifies SHA-256 and the exact expected part length, then saves its provider receipt.
    /// Retry the same part/bytes after a lost response; persist this replacement checkpoint.
    pub async fn upload_part(
        &self,
        checkpoint: &MultipartCheckpoint,
        number: u16,
        bytes: &[u8],
        sha256: [u8; 32],
    ) -> Result<MultipartCheckpoint, MultipartError> {
        let mut state = self.open(checkpoint, false)?;
        if self.part_size(&state, number)? != bytes.len() as u64 {
            return Err(MultipartError::InvalidInput);
        }
        let actual = ring::digest::digest(&ring::digest::SHA256, bytes);
        if !bool::from(actual.as_ref().ct_eq(&sha256)) {
            return Err(MultipartError::Checksum);
        }
        let response = self
            .before_expiry(
                &state,
                self.client.multipart_request(
                    Method::PUT,
                    &self.object,
                    &[
                        ("uploadId", state.upload.clone()),
                        ("partNumber", number.to_string()),
                    ],
                    HeaderMap::new(),
                    bytes,
                ),
            )
            .await?;
        response.expect(200)?;
        let etag = response
            .headers
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .ok_or(MultipartError::InvalidResponse)?;
        if !protocol::valid_etag(etag) || response.headers.get_all("etag").iter().count() != 1 {
            return Err(MultipartError::InvalidResponse);
        }
        self.open(checkpoint, false)?;
        state.receipts.insert(
            number,
            Receipt {
                etag: etag.into(),
                size: bytes.len() as u64,
                sha256,
            },
        );
        self.seal(&state)
    }

    /// Lists every expected part, reporting absent/changed receipts without trusting new bytes.
    /// Unknown provider parts require re-upload to obtain an authenticated receipt.
    pub async fn progress(
        &self,
        checkpoint: &MultipartCheckpoint,
    ) -> Result<Vec<MultipartPart>, MultipartError> {
        let state = self.open(checkpoint, false)?;
        let response = self
            .before_expiry(
                &state,
                self.client.multipart_request(
                    Method::GET,
                    &self.object,
                    &[
                        ("uploadId", state.upload.clone()),
                        ("max-parts", "257".into()),
                    ],
                    HeaderMap::new(),
                    &[],
                ),
            )
            .await?;
        response.expect(200)?;
        let parts = protocol::parts(
            &response.body,
            &self.client.bucket,
            &self.object,
            &state.upload,
        )?;
        if parts.iter().any(|p| p.number > self.part_count(&state)) {
            return Err(MultipartError::InvalidResponse);
        }
        self.open(checkpoint, false)?;
        Ok((1..=self.part_count(&state))
            .map(|number| {
                let part = parts.iter().find(|p| p.number == number);
                let matches_checkpoint = part.is_some_and(|p| {
                    state
                        .receipts
                        .get(&number)
                        .is_some_and(|r| r.size == p.size && r.etag == p.etag)
                });
                MultipartPart {
                    number,
                    size_bytes: part.map_or(0, |p| p.size),
                    matches_checkpoint,
                }
            })
            .collect())
    }

    /// Completes all parts in order. A transport/response failure can be uncertain;
    /// retain the checkpoint and call `reconcile_completion` before another attempt.
    pub async fn complete(&self, checkpoint: &MultipartCheckpoint) -> Result<(), MultipartError> {
        let state = self.open(checkpoint, false)?;
        if state.receipts.len() != usize::from(self.part_count(&state)) {
            return Err(MultipartError::Incomplete);
        }
        let mut xml = String::from("<CompleteMultipartUpload>");
        for (number, receipt) in &state.receipts {
            xml.push_str(&format!(
                "<Part><PartNumber>{number}</PartNumber><ETag>{}</ETag></Part>",
                protocol::escaped(&receipt.etag)
            ));
        }
        xml.push_str("</CompleteMultipartUpload>");
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/xml"));
        let response = self
            .before_expiry(
                &state,
                self.client.multipart_request(
                    Method::POST,
                    &self.object,
                    &[("uploadId", state.upload.clone())],
                    headers,
                    xml.as_bytes(),
                ),
            )
            .await
            .map_err(|_| MultipartError::CompletionUncertain)?;
        response
            .expect(200)
            .map_err(|_| MultipartError::CompletionUncertain)?;
        let doc = protocol::parse(&response.body, "CompleteMultipartUploadResult")
            .map_err(|_| MultipartError::CompletionUncertain)?;
        let root = doc.root_element();
        if protocol::field(root, "Bucket").ok() != Some(&self.client.bucket)
            || protocol::field(root, "Key").ok() != Some(&self.object)
            || !protocol::field(root, "ETag").is_ok_and(protocol::valid_etag)
        {
            return Err(MultipartError::CompletionUncertain);
        }
        self.open(checkpoint, false)
            .map_err(|_| MultipartError::CompletionUncertain)?;
        Ok(())
    }

    /// Checks exact length and the random server-issued upload marker after a lost response.
    /// This does not scan content, authorize reads or prove absence of later replacement.
    pub async fn reconcile_completion(
        &self,
        checkpoint: &MultipartCheckpoint,
    ) -> Result<CompletionStatus, MultipartError> {
        let state = self.open(checkpoint, true)?;
        let response = self
            .client
            .multipart_request(Method::HEAD, &self.object, &[], HeaderMap::new(), &[])
            .await?;
        if response.status == 404 {
            return Ok(CompletionStatus::Unconfirmed);
        }
        response.expect(200)?;
        let marker = response
            .headers
            .get("x-amz-meta-rullst-upload")
            .and_then(|h| h.to_str().ok());
        let size = response
            .headers
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        Ok(
            if marker == Some(state.marker.as_str()) && size == Some(state.total) {
                CompletionStatus::Confirmed
            } else {
                CompletionStatus::Unconfirmed
            },
        )
    }

    /// Aborts an authentic upload, including an expired one, and checks provider absence.
    /// Keep the cleanup record on errors/RetryRequired; stop concurrent part writers first.
    pub async fn abort(
        &self,
        checkpoint: &MultipartCheckpoint,
    ) -> Result<AbortStatus, MultipartError> {
        let state = self.open(checkpoint, true)?;
        let query = [("uploadId", state.upload.clone())];
        let response = self
            .client
            .multipart_request(Method::DELETE, &self.object, &query, HeaderMap::new(), &[])
            .await?;
        if response.status != 404 {
            response.expect(204)?;
        }
        let check = self
            .client
            .multipart_request(Method::GET, &self.object, &query, HeaderMap::new(), &[])
            .await?;
        if check.status == 404 {
            Ok(AbortStatus::Gone)
        } else {
            check.expect(200)?;
            Ok(AbortStatus::RetryRequired)
        }
    }

    async fn before_expiry<T>(
        &self,
        state: &State,
        future: impl Future<Output = Result<T, MultipartError>>,
    ) -> Result<T, MultipartError> {
        let remaining = UNIX_EPOCH
            .checked_add(Duration::from_secs(state.expires))
            .and_then(|deadline| deadline.duration_since(SystemTime::now()).ok())
            .filter(|duration| !duration.is_zero())
            .ok_or(MultipartError::Expired)?;
        tokio::time::timeout(remaining, future)
            .await
            .map_err(|_| MultipartError::Cloud(CloudError::Timeout))?
    }
}
