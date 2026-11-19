### Removed

- **Breaking:** the `experimental-simulators` feature of `rullst-iot` and the
  types behind it, `SimulatedHsmDevice`, `SimulatedHsmProfile`,
  `SimulatedPqcFixture` and `SimulatedMqttPayloadFormatter` (with the `hsm` and
  `pqc` modules), were removed. They only hashed bytes with SHA-256 or
  formatted a number, implemented no HSM, post-quantum cryptography or MQTT
  client, and their tests only tested themselves. Cargo now refuses the
  feature: remove it from the `rullst-iot` dependency and give a test that
  used a fixture its own fixed bytes. `cargo rullst upgrade` reports the
  feature in manifests and the type names in code
  (`V13-IOT-SIMULATORS-REMOVED`).
