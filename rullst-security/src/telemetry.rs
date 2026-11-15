//! Process-local security telemetry that the layers of this crate record into
//! and Studio and Nexus display.
//!
//! Since 13.0.0-alpha.6 the store and its event type are defined in
//! `rullst_core::security`, so the dashboards read them without depending on
//! this crate. The items below re-export them at their earlier paths: both
//! paths name the same types and the same [`SecurityStore::global`] instance.

pub use rullst_core::security::{
    BannedIpRecord, LiveSecurityEvent, SECURITY_EVENT_SCHEMA_VERSION, SecurityStore,
    TelemetrySnapshot, normalize_ip,
};

/// Machine-readable JSON Schema 2020-12 contract for [`LiveSecurityEvent`] v1.
pub const LIVE_SECURITY_EVENT_V1_JSON_SCHEMA: &str =
    include_str!("../schema/security-event-v1.schema.json");

/// Another name for [`SecurityStore`].
pub type SecurityTelemetry = SecurityStore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_json_schema_declares_the_exact_version_one_field_set() {
        let schema: serde_json::Value =
            serde_json::from_str(LIVE_SECURITY_EVENT_V1_JSON_SCHEMA).expect("valid JSON Schema");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        assert_eq!(SECURITY_EVENT_SCHEMA_VERSION, 1);
        let required = schema["required"]
            .as_array()
            .expect("required array")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let serialized = serde_json::to_value(LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "details",
            "unknown",
        ))
        .expect("serialize event");
        let fields = serialized
            .as_object()
            .expect("event object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(required, fields);
    }

    #[test]
    fn the_reexported_store_is_the_core_store() {
        assert!(std::ptr::eq(
            SecurityStore::global(),
            rullst_core::security::SecurityStore::global()
        ));
        let alias: &SecurityTelemetry = SecurityStore::global();
        assert!(std::ptr::eq(alias, SecurityStore::global()));
    }
}
