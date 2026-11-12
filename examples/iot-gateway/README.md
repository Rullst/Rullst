# IoT gateway example

Arduino Uno boards stream sensor readings to a Rullst server, each with its own
bearer token, and a dashboard shows every board live.

This is an example to copy from, not a supported starter. `rullst::iot_ingest`
is Experimental.

```text
Uno "uno-a" --USB serial--> gateway uno-a --HTTP + token A--+
                                                            +--> /iot/telemetry --> channel --> dashboard (SSE)
Uno "uno-b" --USB serial--> gateway uno-b --HTTP + token B--+         |
                                                                 iot_devices (SQLite, Nexus)
```

## What it shows

- **Per-device authentication.** `rullst::iot_ingest::TelemetryReceiver`
  receives `SensorTelemetry` JSON at `POST /iot/telemetry`, registered as a
  machine endpoint: one `Authorization: Bearer` token per device, checked
  against the registry on every request, so a revoked token is refused from
  the next request on and one board cannot post another board's readings
  (`403`). Bodies, readings per request, timestamps and each device's request
  rate are bounded.
- **An application-owned registry.** `src/registry.rs` is the `iot_devices`
  ORM model (device id, name, SHA-256 of the token, firmware version, last
  seen, registered and revoked times), its migration and the
  `DeviceDirectory` implementation the receiver uses. Change it freely: it is
  this application's table, not a Rullst schema. The model derives `Nexus`,
  so `/nexus` lists devices and can set `revoked_at`; the token digest is
  hidden there and in the model's JSON.
- **A live dashboard.** Every accepted request reaches a `BroadcastManager`
  channel; the server folds it into per-device cards and a history table and
  pushes both with `RealtimeSse` to the htmx SSE extension, under the strict
  Content Security Policy.
- **A gateway per board.** `gateway` reads one serial port, checks each
  line's checksum and device id, and posts the readings with that board's
  token. It drops stale bytes when the port opens, reopens the port after
  errors, pauses after `429` and reports a refused token.

## The boards

`sketch/uno_sensor/uno_sensor.ino` prints one line per second at 115200 baud:

```text
# rullst-uno-sensor fw=2 dev=uno-a
dev=uno-a seq=12 temp_raw=333 temp_c=7.1 sine=0.59 xor=3F
```

`temp_c` is the ATmega328P's internal sensor with a generic formula: it is
uncalibrated and can be 10–20 °C off. `sine` is synthetic. Compile one image
per board with its own `DEVICE_ID` (here with
[arduino-cli](https://arduino.github.io/arduino-cli/) and the `arduino:avr`
core):

```bash
arduino-cli compile --fqbn arduino:avr:uno --output-dir build/uno-a \
  --build-property 'compiler.cpp.extra_flags=-DDEVICE_ID="uno-a"' sketch/uno_sensor
arduino-cli upload --fqbn arduino:avr:uno -p /dev/ttyUSB0 --input-dir build/uno-a
arduino-cli compile --fqbn arduino:avr:uno --output-dir build/uno-b \
  --build-property 'compiler.cpp.extra_flags=-DDEVICE_ID="uno-b"' sketch/uno_sensor
arduino-cli upload --fqbn arduino:avr:uno -p /dev/ttyUSB1 --input-dir build/uno-b
```

Boards with a CH340 USB chip appear as `/dev/ttyUSB*` on Linux (`COM*` on
Windows); your user needs access to them (the `dialout` group on most Linux
distributions). Sending the byte `g` to a board makes it also emit five
malformed lines once, to exercise the gateway's rejection path.

## Run it

From a checkout of this repository, in this directory (the server serves
`static/` from the working directory):

```bash
cd examples/iot-gateway
cargo run -p rullst-iot-gateway-example -- register uno-a "Greenhouse"   # prints token A once
cargo run -p rullst-iot-gateway-example -- register uno-b "Workshop"     # prints token B once
cargo run -p rullst-iot-gateway-example -- server
```

In two more terminals, one gateway per board:

```bash
IOT_DEVICE_TOKEN=<token A> cargo run -p rullst-iot-gateway-example -- gateway /dev/ttyUSB0 uno-a
IOT_DEVICE_TOKEN=<token B> cargo run -p rullst-iot-gateway-example -- gateway /dev/ttyUSB1 uno-b
```

Open <http://127.0.0.1:3000>. Revoke a board and its gateway's posts are
refused at once while the other board keeps streaming:

```bash
cargo run -p rullst-iot-gateway-example -- revoke uno-b
cargo run -p rullst-iot-gateway-example -- devices
```

The database is `DATABASE_URL`, else `iot-gateway.db` here. The gateway posts
to `http://127.0.0.1:3000` unless a third argument names the server. A token
is shown only when it is created; register a new device id to replace a lost
one. In debug builds `/nexus` is loopback-only; release builds need
`NEXUS_ADMIN_USERNAME` and `NEXUS_ADMIN_PASSWORD` and otherwise start
without it. Set `RULLST_ENV=production` to run with the production security
headers.

## Tests

```bash
cargo test -p rullst-iot-gateway-example
```

They need no hardware: the parser and gateway decisions, the dashboard views,
the registry on SQLite with its Nexus metadata, and two devices posting
through the production security baseline with one token revoked. Under
`cargo test --workspace --all-features` the ORM is compiled for PostgreSQL,
so the SQLite tests skip; CI runs them in the IoT workflow.

## Validation on hardware

On 2026-11-11 two Arduino Uno clones (CH340) on one USB hub ran this example:
both boards streamed concurrently through their gateways with separate
registry entries, the dashboard showed both in headless Chrome without CSP
violations, and a revoked token was refused while the other board continued.

To use it outside this repository, replace the `workspace = true`
dependencies with published versions, for example
`rullst = { version = "=13.0.0-alpha.5", features = ["orm", "strict-sqlite", "nexus", "iot-ingest"] }`.
