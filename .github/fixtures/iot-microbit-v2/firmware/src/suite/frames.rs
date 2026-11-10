//! Telemetry JSON, digital-twin snapshots, Modbus and I2C frames.

use super::Ctx;
use crate::expected as x;
use rullst_iot::{
    DigitalTwin, I2cFrameError, I2cHelper, ModbusFrame, SensorTelemetry, TwinSyncError,
};

pub(super) fn frames(ctx: &mut Ctx) {
    let telemetry = SensorTelemetry::new("microbit-v2-01", "temperature", 23.75, 1_762_790_400);
    match serde_json::to_vec(&telemetry) {
        Ok(json) => ctx.bytes("telemetry_json_encode", &json, x::TELEMETRY_JSON),
        Err(_) => ctx.check(
            "telemetry_json_encode",
            false,
            format_args!("serde_json error"),
        ),
    }
    match serde_json::from_slice::<SensorTelemetry>(x::TELEMETRY_JSON) {
        Ok(t) => ctx.check(
            "telemetry_json_decode",
            t.device_id == "microbit-v2-01"
                && t.metric == "temperature"
                && t.value == 23.75
                && t.timestamp == 1_762_790_400,
            format_args!(
                "device_id={} metric={} value={} ts={}",
                t.device_id, t.metric, t.value, t.timestamp
            ),
        ),
        Err(_) => ctx.check("telemetry_json_decode", false, format_args!("decode error")),
    }
    let truncated = &x::TELEMETRY_JSON[..x::TELEMETRY_JSON.len() - 3];
    let wrong_type = br#"{"device_id":"d","metric":"m","value":"hot","timestamp":1}"#;
    ctx.check(
        "telemetry_json_reject_malformed",
        serde_json::from_slice::<SensorTelemetry>(truncated).is_err()
            && serde_json::from_slice::<SensorTelemetry>(wrong_type).is_err(),
        format_args!("truncated and string-valued frames rejected"),
    );

    let mut twin = DigitalTwin::new("microbit-v2-01");
    twin.ingest(telemetry.clone());
    twin.ingest(SensorTelemetry::new(
        "microbit-v2-01",
        "light",
        0.125,
        1_762_790_401,
    ));
    match twin.try_to_sync_payload() {
        Ok(json) => ctx.bytes("twin_sync_payload", json.as_bytes(), x::TWIN_JSON),
        Err(e) => ctx.check("twin_sync_payload", false, format_args!("{}", e)),
    }
    let latest_ok = twin.latest("temperature").map(|t| t.value) == Some(23.75)
        && twin.latest("light").map(|t| t.timestamp) == Some(1_762_790_401)
        && twin.latest("missing").is_none();
    ctx.check("twin_latest", latest_ok, format_args!("latest per metric"));
    let decoded = serde_json::from_slice::<DigitalTwin>(x::TWIN_JSON);
    ctx.check(
        "twin_json_decode",
        matches!(&decoded, Ok(t) if t.readings.len() == 2 && t.is_online),
        format_args!("host snapshot decoded on device"),
    );
    twin.ingest(SensorTelemetry::new(
        "microbit-v2-01",
        "temperature",
        f64::NAN,
        1,
    ));
    let non_finite = matches!(
        twin.try_to_sync_payload(),
        Err(TwinSyncError::NonFiniteReading)
    );
    let fallback = twin.to_sync_payload();
    ctx.check(
        "twin_non_finite_fails_closed",
        non_finite && fallback == "{\"error\":\"serialize_failed\"}",
        format_args!("NaN -> NonFiniteReading, fallback={}", fallback),
    );

    let modbus = ModbusFrame::read_holding_registers(17, 0x006B, 3);
    ctx.bytes("modbus_rtu_frame", &modbus, x::MODBUS_READ_17_6B_3);
    let crc_ok = modbus.len() == 8
        && ModbusFrame::calculate_crc16(&modbus[..6]) == u16::from_le_bytes([modbus[6], modbus[7]])
        && ModbusFrame::calculate_crc16(&modbus) == 0
        && ModbusFrame::calculate_crc16(b"123456789") == 0x4B37;
    ctx.check(
        "modbus_crc_validate",
        crc_ok,
        format_args!("crc(frame||crc)==0, check vector 0x4B37"),
    );

    match I2cHelper::try_build_read_frame(0x19, 0x0F, 6) {
        Ok(frame) => ctx.bytes("i2c_read_frame", &frame, x::I2C_READ_19_0F_6),
        Err(e) => ctx.check("i2c_read_frame", false, format_args!("{:?}", e)),
    }
    ctx.expect_err(
        "i2c_reserved_address",
        I2cHelper::try_build_read_frame(0x78, 0, 1),
        I2cFrameError::InvalidAddress,
    );
    ctx.expect_err(
        "i2c_read_too_long",
        I2cHelper::try_build_read_frame(0x19, 0, 8193),
        I2cFrameError::ReadTooLong,
    );
}
