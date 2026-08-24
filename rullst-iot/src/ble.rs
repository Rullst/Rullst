//! Bluetooth Low Energy GATT service and characteristic data structures.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

/// BLE GATT characteristic data. This type does not operate a radio.
#[derive(Clone, Debug)]
pub struct GattCharacteristic {
    pub uuid: String,
    pub value: Vec<u8>,
    pub notify: bool,
}

/// BLE GATT service data. This type does not run a GATT server.
#[derive(Clone, Debug)]
pub struct GattService {
    pub service_uuid: String,
    pub characteristics: Vec<GattCharacteristic>,
}

impl GattService {
    /// Creates a new GATT Service.
    pub fn new(uuid: impl Into<String>) -> Self {
        Self {
            service_uuid: uuid.into(),
            characteristics: Vec::new(),
        }
    }

    /// Adds a telemetry characteristic to the GATT service.
    pub fn add_characteristic(&mut self, uuid: impl Into<String>, value: Vec<u8>, notify: bool) {
        self.characteristics.push(GattCharacteristic {
            uuid: uuid.into(),
            value,
            notify,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_gatt_service_builder() {
        let mut service = GattService::new("180F"); // Battery Service
        service.add_characteristic("2A19", vec![100], true);

        assert_eq!(service.service_uuid, "180F");
        assert_eq!(service.characteristics.len(), 1);
        assert_eq!(service.characteristics[0].value[0], 100);
    }
}
