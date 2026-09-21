use rullst_labs::LabError as Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(super) async fn read_frame<T: serde::de::DeserializeOwned>(
    reader: &mut (impl AsyncRead + Unpin),
    max: usize,
) -> Result<T, Error> {
    let length = reader.read_u32().await.map_err(|_| Error::Protocol)? as usize;
    if length == 0 || length > max {
        return Err(Error::Capacity);
    }
    let mut bytes = zeroize::Zeroizing::new(vec![0; length]);
    reader
        .read_exact(&mut bytes)
        .await
        .map_err(|_| Error::Protocol)?;
    serde_json::from_slice(&bytes).map_err(|_| Error::Protocol)
}
pub(super) async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    value: &impl serde::Serialize,
) -> Result<(), Error> {
    let bytes = zeroize::Zeroizing::new(serde_json::to_vec(value).map_err(|_| Error::Protocol)?);
    if bytes.is_empty() || bytes.len() > 131_072 {
        return Err(Error::Capacity);
    }
    writer
        .write_u32(u32::try_from(bytes.len()).map_err(|_| Error::Capacity)?)
        .await
        .map_err(|_| Error::Protocol)?;
    writer
        .write_all(&bytes)
        .await
        .map_err(|_| Error::Protocol)?;
    writer.flush().await.map_err(|_| Error::Protocol)
}
