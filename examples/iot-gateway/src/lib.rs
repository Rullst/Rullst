//! IoT gateway example: Arduino Uno boards stream readings over USB serial;
//! a gateway process per board posts them, with the board's own bearer
//! token, to the server's `rullst::iot_ingest::TelemetryReceiver`; the
//! server keeps the device registry in its database (an ORM model edited in
//! Nexus) and pushes a live dashboard over Server-Sent Events.
//!
//! An example to copy from, not a supported starter; see `README.md`.

pub mod dashboard;
pub mod gateway;
pub mod line;
pub mod registry;
pub mod server;

/// Writes one diagnostic line to stderr; a closed stderr drops it instead of
/// panicking as `eprintln!` would.
pub fn note(line: std::fmt::Arguments<'_>) {
    use std::io::Write;
    let mut stderr = std::io::stderr().lock();
    let _ = stderr
        .write_fmt(line)
        .and_then(|()| stderr.write_all(b"\n"));
}

#[cfg(test)]
mod tests;
