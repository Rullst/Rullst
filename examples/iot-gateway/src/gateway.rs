//! The edge gateway: reads one board's serial port and posts each reading to
//! the server's telemetry receiver with that device's bearer token.

use std::io::Read;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rullst::iot::SensorTelemetry;
use rullst::iot_ingest::FIRMWARE_HEADER;
use serialport::SerialPort as _;
use tokio::sync::mpsc;

use crate::line::{MAX_LINE_BYTES, Parsed, Reject, SensorLine, parse_line};

/// Where and as whom a gateway posts.
#[derive(Clone, Debug)]
pub struct GatewayConfig {
    pub serial_port: String,
    pub baud: u32,
    pub device_id: String,
    pub endpoint: String,
    pub token: String,
}

/// One serial line and its full length (the bytes stop at the parser limit).
struct RawLine {
    bytes: Vec<u8>,
    length: usize,
}

/// What the gateway does with one parsed line.
#[derive(Debug)]
pub enum Action {
    /// Post these readings.
    Post(Vec<SensorTelemetry>),
    /// Remember the firmware version the board announced.
    Firmware(String),
    /// Nothing to send.
    Skip(String),
}

/// Turns one serial line into an [`Action`] for `device_id` at `now`.
pub fn plan(bytes: &[u8], length: usize, device_id: &str, now: u64) -> Action {
    let parsed = if length > MAX_LINE_BYTES {
        Err(Reject::TooLong(length))
    } else {
        parse_line(bytes)
    };
    match parsed {
        Ok(Parsed::Reading(line)) if line.device_id == device_id => {
            Action::Post(readings(&line, now))
        }
        Ok(Parsed::Reading(line)) => Action::Skip(format!(
            "line from {} on the port of {device_id}",
            line.device_id
        )),
        Ok(Parsed::Info {
            firmware: Some(firmware),
            ..
        }) => Action::Firmware(firmware),
        Ok(Parsed::Info { text, .. }) => Action::Skip(format!("info: {text}")),
        Err(reason) => Action::Skip(format!("rejected: {reason}")),
    }
}

/// The readings of one line, as `rullst-iot` telemetry.
pub fn readings(line: &SensorLine, now: u64) -> Vec<SensorTelemetry> {
    let device = line.device_id.as_str();
    vec![
        SensorTelemetry::new(device, "temperature_c_uncalibrated", line.temp_c, now),
        SensorTelemetry::new(device, "temperature_adc_raw", f64::from(line.temp_raw), now),
        SensorTelemetry::new(device, "sine", line.sine, now),
        SensorTelemetry::new(device, "sequence", f64::from(line.seq), now),
    ]
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Reads the port on a thread, reopening it after errors, and sends every
/// complete line. Bytes queued before the port opened are discarded.
#[cfg_attr(mutants, mutants::skip)]
fn spawn_reader(path: String, baud: u32, lines: mpsc::Sender<RawLine>) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name(format!("serial {path}"))
        .spawn(move || {
            loop {
                let port = serialport::new(&path, baud)
                    .timeout(Duration::from_millis(250))
                    .open_native();
                let mut port = match port {
                    Ok(port) => port,
                    Err(error) => {
                        crate::note(format_args!(
                            "gateway: cannot open {path}: {error}; retrying in 2 s"
                        ));
                        std::thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };
                let _ = port.clear(serialport::ClearBuffer::Input);
                crate::note(format_args!("gateway: reading {path} at {baud} baud"));
                if let Err(error) = read_lines(&mut port, &lines) {
                    crate::note(format_args!("gateway: {path}: {error}; reopening"));
                    std::thread::sleep(Duration::from_secs(1));
                }
                if lines.is_closed() {
                    return;
                }
            }
        })
        .map(|_| ())
}

/// Sends complete lines until the port fails or the receiver is gone. A line
/// never grows past the parser's limit; one cut by the port opening mid-line
/// fails its checksum and is rejected like any garbled line.
#[cfg_attr(mutants, mutants::skip)]
fn read_lines(port: &mut impl Read, lines: &mpsc::Sender<RawLine>) -> std::io::Result<()> {
    let mut line = Vec::with_capacity(MAX_LINE_BYTES + 1);
    let mut length = 0_usize;
    let mut chunk = [0_u8; 256];
    loop {
        let read = match port.read(&mut chunk) {
            Ok(0) => return Err(std::io::ErrorKind::UnexpectedEof.into()),
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        for &byte in &chunk[..read] {
            if byte == b'\n' {
                let raw = RawLine {
                    bytes: std::mem::take(&mut line),
                    length,
                };
                if lines.blocking_send(raw).is_err() {
                    return Ok(());
                }
                length = 0;
            } else {
                length = length.saturating_add(1);
                if line.len() <= MAX_LINE_BYTES {
                    line.push(byte);
                }
            }
        }
    }
}

/// Runs the gateway until the process stops.
#[cfg_attr(mutants, mutants::skip)]
pub async fn run(config: GatewayConfig) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut lines) = mpsc::channel(64);
    spawn_reader(config.serial_port.clone(), config.baud, sender)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let mut firmware: Option<String> = None;
    let mut paused_until: Option<Instant> = None;
    while let Some(raw) = lines.recv().await {
        let readings = match plan(&raw.bytes, raw.length, &config.device_id, unix_now()) {
            Action::Post(readings) => readings,
            Action::Firmware(version) => {
                crate::note(format_args!("gateway: board announced firmware {version}"));
                firmware = Some(version);
                continue;
            }
            Action::Skip(reason) => {
                crate::note(format_args!("gateway: {reason}"));
                continue;
            }
        };
        if paused_until.is_some_and(|until| Instant::now() < until) {
            continue;
        }
        let mut request = client
            .post(&config.endpoint)
            .bearer_auth(&config.token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_vec(&readings)?);
        if let Some(version) = &firmware {
            request = request.header(FIRMWARE_HEADER, version);
        }
        match request.send().await {
            Ok(response) if response.status() == reqwest::StatusCode::ACCEPTED => {}
            Ok(response) if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                let wait = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(5);
                crate::note(format_args!("gateway: rate limited; pausing {wait} s"));
                paused_until = Some(Instant::now() + Duration::from_secs(wait));
            }
            Ok(response) if response.status() == reqwest::StatusCode::UNAUTHORIZED => {
                crate::note(format_args!(
                    "gateway: server refused the token of {} (unknown or revoked)",
                    config.device_id
                ));
            }
            Ok(response) => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                crate::note(format_args!("gateway: server answered {status}: {text}"));
            }
            Err(error) => crate::note(format_args!("gateway: cannot reach the server: {error}")),
        }
    }
    Ok(())
}
