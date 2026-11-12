//! Strict parser for the sketch's serial lines.
//!
//! A reading is `dev=uno-a seq=12 temp_raw=333 temp_c=7.1 sine=0.59 xor=3F`,
//! where `xor` is the XOR of every byte before ` xor=`. A line starting with
//! `#` is informational; `# rullst-uno-sensor fw=2 dev=uno-a` announces the
//! firmware after a reset. Anything else is rejected with a reason.

use std::fmt;

/// Longest accepted line, without its terminator.
pub const MAX_LINE_BYTES: usize = 120;

/// One decoded reading.
#[derive(Clone, Debug, PartialEq)]
pub struct SensorLine {
    pub device_id: String,
    pub seq: u32,
    pub temp_raw: u16,
    pub temp_c: f64,
    pub sine: f64,
}

/// What a line turned out to be.
#[derive(Clone, Debug, PartialEq)]
pub enum Parsed {
    Reading(SensorLine),
    /// An informational line; `firmware` is set when it announces one.
    Info {
        text: String,
        firmware: Option<String>,
    },
}

/// Why a line was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reject {
    Empty,
    TooLong(usize),
    NotUtf8,
    NoChecksum,
    BadChecksum,
    Field(&'static str),
    OutOfRange(&'static str),
    Trailing,
}

impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty line"),
            Self::TooLong(n) => write!(f, "line longer than {MAX_LINE_BYTES} bytes ({n})"),
            Self::NotUtf8 => f.write_str("line is not UTF-8"),
            Self::NoChecksum => f.write_str("missing xor checksum"),
            Self::BadChecksum => f.write_str("checksum mismatch"),
            Self::Field(name) => write!(f, "missing or malformed field `{name}`"),
            Self::OutOfRange(name) => write!(f, "field `{name}` out of range"),
            Self::Trailing => f.write_str("unexpected trailing data"),
        }
    }
}

/// Parses one line whose `\n` was removed (a trailing `\r` is ignored).
pub fn parse_line(bytes: &[u8]) -> Result<Parsed, Reject> {
    if bytes.len() > MAX_LINE_BYTES {
        return Err(Reject::TooLong(bytes.len()));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Reject::NotUtf8)?;
    let text = text.trim_end_matches('\r');
    if text.is_empty() {
        return Err(Reject::Empty);
    }
    if let Some(info) = text.strip_prefix('#') {
        if info.chars().any(char::is_control) {
            return Err(Reject::Field("info"));
        }
        let info = info.trim();
        let firmware = info
            .split(' ')
            .find_map(|word| word.strip_prefix("fw="))
            .filter(|version| !version.is_empty())
            .map(|version| format!("rullst-uno-sensor-{version}"));
        return Ok(Parsed::Info {
            text: info.to_owned(),
            firmware,
        });
    }
    let (body, check) = text.rsplit_once(" xor=").ok_or(Reject::NoChecksum)?;
    let expected = body.bytes().fold(0_u8, |acc, byte| acc ^ byte);
    let valid_hex = check.len() == 2 && check.bytes().all(|b| b.is_ascii_hexdigit());
    if !valid_hex || u8::from_str_radix(check, 16).ok() != Some(expected) {
        return Err(Reject::BadChecksum);
    }

    let mut fields = body.split(' ');
    let device_id = value(fields.next(), "dev")?;
    if !(1..=64).contains(&device_id.len())
        || !device_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(Reject::Field("dev"));
    }
    let seq: u32 = number(fields.next(), "seq")?;
    let temp_raw: u16 = number(fields.next(), "temp_raw")?;
    let temp_c: f64 = number(fields.next(), "temp_c")?;
    let sine: f64 = number(fields.next(), "sine")?;
    if fields.next().is_some() {
        return Err(Reject::Trailing);
    }
    if temp_raw > 1023 {
        return Err(Reject::OutOfRange("temp_raw"));
    }
    if !(-60.0..=150.0).contains(&temp_c) {
        return Err(Reject::OutOfRange("temp_c"));
    }
    if !(-1.0..=1.0).contains(&sine) {
        return Err(Reject::OutOfRange("sine"));
    }
    Ok(Parsed::Reading(SensorLine {
        device_id: device_id.to_owned(),
        seq,
        temp_raw,
        temp_c,
        sine,
    }))
}

fn value<'a>(part: Option<&'a str>, name: &'static str) -> Result<&'a str, Reject> {
    part.and_then(|part| part.strip_prefix(name))
        .and_then(|rest| rest.strip_prefix('='))
        .filter(|value| !value.is_empty())
        .ok_or(Reject::Field(name))
}

fn number<T: std::str::FromStr>(part: Option<&str>, name: &'static str) -> Result<T, Reject> {
    let text = value(part, name)?;
    // Plain decimal only: no `inf`, `nan`, exponent or leading `+`.
    let plain = text
        .bytes()
        .enumerate()
        .all(|(i, b)| b.is_ascii_digit() || b == b'.' || (i == 0 && b == b'-'));
    if !plain {
        return Err(Reject::Field(name));
    }
    text.parse().map_err(|_| Reject::Field(name))
}
