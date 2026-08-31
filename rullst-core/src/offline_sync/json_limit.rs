use serde::Serialize;
use std::io::Write;

pub(super) enum BoundedJsonError {
    LimitExceeded,
    Encode(serde_json::Error),
}

pub(super) fn encode_bounded<T>(value: &T, maximum: usize) -> Result<Vec<u8>, BoundedJsonError>
where
    T: Serialize,
{
    let mut writer = BoundedJsonWriter::new(maximum);
    match serde_json::to_writer(&mut writer, value) {
        Ok(()) => Ok(writer.into_bytes()),
        Err(_) if writer.exceeded() => Err(BoundedJsonError::LimitExceeded),
        Err(error) => Err(BoundedJsonError::Encode(error)),
    }
}

pub(super) fn validate_bounded<T>(value: &T, maximum: usize) -> Result<(), BoundedJsonError>
where
    T: Serialize,
{
    let mut writer = JsonSizeWriter::new(maximum);
    match serde_json::to_writer(&mut writer, value) {
        Ok(()) => Ok(()),
        Err(_) if writer.exceeded() => Err(BoundedJsonError::LimitExceeded),
        Err(error) => Err(BoundedJsonError::Encode(error)),
    }
}

struct BoundedJsonWriter {
    bytes: Vec<u8>,
    maximum: usize,
    exceeded: bool,
}

impl BoundedJsonWriter {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
            exceeded: false,
        }
    }

    const fn exceeded(&self) -> bool {
        self.exceeded
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedJsonWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.bytes.len());
        if buffer.len() > remaining {
            self.exceeded = true;
            return Err(std::io::Error::other("offline JSON limit reached"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct JsonSizeWriter {
    written: usize,
    maximum: usize,
    exceeded: bool,
}

impl JsonSizeWriter {
    const fn new(maximum: usize) -> Self {
        Self {
            written: 0,
            maximum,
            exceeded: false,
        }
    }

    const fn exceeded(&self) -> bool {
        self.exceeded
    }
}

impl Write for JsonSizeWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let Some(written) = self.written.checked_add(buffer.len()) else {
            self.exceeded = true;
            return Err(std::io::Error::other("offline JSON limit reached"));
        };
        if written > self.maximum {
            self.exceeded = true;
            return Err(std::io::Error::other("offline JSON limit reached"));
        }
        self.written = written;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
