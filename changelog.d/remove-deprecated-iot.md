### Removed

- **Breaking:** `rullst-iot`'s `OtaManager::new`, `OtaManager::verify_signature`
  and `OtaManager::commit_update`, deprecated in 12.0.0, were removed with
  `OtaError::LegacyApiUnsupported`, the error all three always returned. Build
  the manager with `OtaManager::new_with_trusted_key` (or
  `new_with_counter_store`), verify with `begin_update` or `verify_update` and
  commit with `commit_verified_update`. `cargo rullst upgrade` reports each use
  (`V13-IOT-OTA-LEGACY-REMOVED`). The micro:bit V2 hardware suite drops its
  `ota_legacy_api_fails_closed` check.
