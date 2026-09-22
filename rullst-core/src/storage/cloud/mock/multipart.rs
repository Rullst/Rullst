use super::super::{
    CloudError,
    multipart::{
        MultipartError,
        protocol::{escaped, field, parse},
        transport::Wire,
    },
};
use super::{MAX_BYTES, MAX_OBJECTS, MockStore};
use reqwest::{
    Method,
    header::{HeaderMap, HeaderValue},
};
use std::collections::BTreeMap;

pub(super) struct Upload {
    key: String,
    marker: String,
    parts: BTreeMap<u16, (String, Vec<u8>)>,
}

fn wire(status: u16, body: impl Into<Vec<u8>>) -> Wire {
    Wire {
        status,
        body: body.into(),
        headers: HeaderMap::new(),
    }
}
fn header(value: &str) -> Result<HeaderValue, MultipartError> {
    HeaderValue::from_str(value).map_err(|_| MultipartError::InvalidResponse)
}

impl MockStore {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn multipart_request(
        &self,
        method: Method,
        bucket: &str,
        key: &str,
        query: &[(&str, String)],
        headers: HeaderMap,
        body: &[u8],
    ) -> Result<Wire, MultipartError> {
        let mut state = self.0.lock().map_err(|_| CloudError::MockUnavailable)?;
        let param = |name: &str| {
            query
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.as_str())
        };
        let resource = format!(
            "<Bucket>{}</Bucket><Key>{}</Key>",
            escaped(bucket),
            escaped(key)
        );
        if method == Method::HEAD {
            let Some(bytes) = state.objects.get(key) else {
                return Ok(wire(404, []));
            };
            let mut response = wire(200, []);
            response
                .headers
                .insert("content-length", header(&bytes.len().to_string())?);
            if let Some(marker) = state.markers.get(key) {
                response
                    .headers
                    .insert("x-amz-meta-rullst-upload", header(marker)?);
            }
            return Ok(response);
        }
        if method == Method::POST && param("uploads").is_some() {
            if state.objects.len() + state.uploads.len() >= MAX_OBJECTS {
                return Err(CloudError::MockUnavailable.into());
            }
            state.next_upload = state
                .next_upload
                .checked_add(1)
                .ok_or(CloudError::MockUnavailable)?;
            let id = format!("mock-upload-{}", state.next_upload);
            let marker = headers
                .get("x-amz-meta-rullst-upload")
                .and_then(|h| h.to_str().ok())
                .ok_or(MultipartError::InvalidInput)?;
            state.uploads.insert(
                id.clone(),
                Upload {
                    key: key.into(),
                    marker: marker.into(),
                    parts: BTreeMap::new(),
                },
            );
            return Ok(wire(
                200,
                format!(
                    "<InitiateMultipartUploadResult>{resource}<UploadId>{id}</UploadId></InitiateMultipartUploadResult>"
                ),
            ));
        }
        let id = param("uploadId").ok_or(MultipartError::InvalidInput)?;
        let Some(upload) = state.uploads.get(id).filter(|u| u.key == key) else {
            return Ok(wire(404, []));
        };
        let used: usize = upload.parts.values().map(|(_, v)| v.len()).sum();
        if method == Method::GET {
            let mut xml = format!(
                "<ListPartsResult>{resource}<UploadId>{}</UploadId><IsTruncated>false</IsTruncated>",
                escaped(id)
            );
            for (number, (etag, bytes)) in &upload.parts {
                xml.push_str(&format!(
                    "<Part><PartNumber>{number}</PartNumber><ETag>{}</ETag><Size>{}</Size></Part>",
                    escaped(etag),
                    bytes.len()
                ));
            }
            xml.push_str("</ListPartsResult>");
            return Ok(wire(200, xml));
        }
        if method == Method::DELETE {
            state.uploads.remove(id);
            state.total -= used;
            return Ok(wire(204, []));
        }
        if method == Method::PUT {
            let number = param("partNumber")
                .and_then(|v| v.parse::<u16>().ok())
                .filter(|v| (1..=256).contains(v))
                .ok_or(MultipartError::InvalidInput)?;
            let old = upload.parts.get(&number).map_or(0, |(_, v)| v.len());
            let total = state
                .total
                .checked_sub(old)
                .and_then(|n| n.checked_add(body.len()))
                .filter(|n| *n <= MAX_BYTES)
                .ok_or(CloudError::MockUnavailable)?;
            let digest = ring::digest::digest(&ring::digest::SHA256, body);
            let mut etag = String::from("\"");
            for byte in digest.as_ref() {
                use std::fmt::Write;
                write!(etag, "{byte:02x}").map_err(|_| MultipartError::InvalidInput)?;
            }
            etag.push('"');
            state
                .uploads
                .get_mut(id)
                .ok_or(CloudError::MockUnavailable)?
                .parts
                .insert(number, (etag.clone(), body.to_vec()));
            state.total = total;
            let mut response = wire(200, []);
            response.headers.insert("etag", header(&etag)?);
            return Ok(response);
        }
        if method == Method::POST {
            let doc = parse(body, "CompleteMultipartUpload")?;
            let mut bytes = Vec::new();
            let mut previous = 0;
            for node in doc.root_element().children().filter(|n| n.is_element()) {
                let number = field(node, "PartNumber")?
                    .parse::<u16>()
                    .map_err(|_| MultipartError::InvalidInput)?;
                let Some((etag, part)) = upload.parts.get(&number) else {
                    return Ok(wire(400, []));
                };
                if number != previous + 1 || field(node, "ETag")? != etag {
                    return Ok(wire(400, []));
                }
                bytes.extend(part);
                previous = number;
            }
            if bytes.is_empty() {
                return Ok(wire(400, []));
            }
            let marker = upload.marker.clone();
            let old = state.objects.get(key).map_or(0, Vec::len);
            state.total = state.total - used - old + bytes.len();
            state.objects.insert(key.into(), bytes);
            state.markers.insert(key.into(), marker);
            state.uploads.remove(id);
            return Ok(wire(
                200,
                format!(
                    "<CompleteMultipartUploadResult>{resource}<ETag>&quot;mock-complete&quot;</ETag></CompleteMultipartUploadResult>"
                ),
            ));
        }
        Err(MultipartError::InvalidInput)
    }
}
