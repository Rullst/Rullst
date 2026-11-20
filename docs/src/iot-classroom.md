# Robotics and IoT in the Classroom

> 🇧🇷 [Leia esta página em português](pt-BR/iot-classroom.md).

This guide is for teachers who want to combine robotics boards with a Rullst
web application in class: which boards to pick, project ideas, and safety
notes. It keeps three kinds of statement apart:

- **Validated with Rullst:** what the project has run on real hardware, with
  the date and a link to the results. Only the BBC micro:bit V2 and the
  Arduino Uno are in this group.
- **Board facts:** connectivity and Rust support, taken from the
  manufacturers' pages and the crates' documentation on 2026-11-19 (see
  [sources](#sources-for-the-board-facts)).
- **Ideas:** every project below. None of them is a shipped example or a
  tested lesson plan. Each one says which Rullst parts it would use and what
  you build or bring yourself.

## What Rullst brings to a robotics class

The boards do the physical work. Rullst runs on a laptop or a small Linux
computer and gives the class a web page: buttons, live readings, history, a
leaderboard. These are the parts the ideas below use:

| Part | What it does | Documentation |
| :--- | :--- | :--- |
| `rullst-iot` | `SensorTelemetry` (one reading: `device_id`, `metric`, `value` and `timestamp` in whole UNIX seconds), `AnomalyDetector`, MQTT 5 PUBLISH and CoAP encoders, and the Ed25519 signed-firmware check (`OtaManager`). It runs on the server and, without its `std` feature, on a board. It has no network code. | [rullst-iot guide](crates/iot.md) |
| `rullst::iot_ingest` (facade feature `iot-ingest`) | `TelemetryReceiver`: an HTTP route that accepts `SensorTelemetry` JSON with one bearer token per device and publishes each accepted batch to a realtime channel (`iot.telemetry` by default). | [Receiving telemetry](crates/iot.md#receiving-telemetry-on-the-server-rullstiot_ingest) |
| Core realtime | `BroadcastManager` (channels), `PresenceTracker` (who is connected) and `RealtimeSse` (Server-Sent Events to a page that uses the htmx SSE extension). | [Tutorial 34](tutorials/34-realtime-dashboard-sse.md), [tutorial 17](tutorials/17-websockets-realtime-chat.md) |
| ORM | Models with `#[derive(Orm)]` and migrations, on SQLite with the facade features `orm` and `strict-sqlite`. | [Tutorial 03](tutorials/03-active-record-crud.md) |
| CLI | `cargo rullst make:iot`, `iot:keygen`, `iot:sign` and `make:model --migration`. | [CLI reference](cli_reference.md#cargo-rullst-iotkeygen) |
| `rullst-labs` | Exercise, submission and exact-grading contracts. You bring the runner that executes students' code. | [Runner contract](labs-runner-contract.md) |

The firmware on a board does not have to be written in Rust. In the validated
two-board run, Arduino Uno boards ran an Arduino C++ sketch, and a gateway
program on the computer read each board's USB serial port and posted the
readings to Rullst. That program is
[`examples/iot-gateway`](https://github.com/Rullst/Rullst/tree/main/examples/iot-gateway),
the closest working code for every idea on this page: per-board tokens, a
device registry in SQLite with a Nexus page, and a live dashboard over
Server-Sent Events. It serves its own htmx files, so its page needs no
internet connection. Nothing in that pattern depends on the board's
programming language, so a micro:bit programmed with MakeCode or Python could
feed the same kind of gateway; that combination has not been tested.

## Boards

Cost levels are relative and approximate, not prices: check local stores.

| Board | Connectivity | Rust support | Validated with Rullst | Relative cost |
| :--- | :--- | :--- | :--- | :--- |
| **BBC micro:bit V2** | micro:bit radio and Bluetooth Low Energy; no Wi-Fi; USB serial to a computer | Community crates: [`microbit-v2`](https://docs.rs/microbit-v2) board crate, [`embassy-nrf`](https://docs.rs/embassy-nrf) | **Yes:** `rullst-iot` ran on the board (2026-11-10) | Low |
| **Arduino Uno R3** | None on the board; USB serial to a computer | Community crate: [`avr-hal`](https://github.com/Rahix/avr-hal) (`arduino-hal`), nightly Rust | **Yes, as a sensor board:** an Arduino sketch fed a Rullst gateway (2026-11-10 and 11); `rullst-iot` ran on the computer | Low |
| **ESP32-C3** | 2.4 GHz Wi-Fi (802.11 b/g/n) and Bluetooth 5 (LE) | [`esp-hal`](https://docs.espressif.com/projects/rust/), backed by Espressif | **No**, waiting for a board ([#637](https://github.com/Rullst/Rullst/issues/637)) | Very low |
| **Raspberry Pi Pico 2 W** | 2.4 GHz Wi-Fi (802.11n) and Bluetooth 5.2 | Community crates: [`embassy-rp`](https://docs.rs/embassy-rp), [`rp235x-hal`](https://docs.rs/rp235x-hal), [`cyw43`](https://docs.rs/cyw43) for the radio | **No** | Very low |
| **ESP32-S3** (also camera boards) | 2.4 GHz Wi-Fi (802.11 b/g/n) and Bluetooth 5 (LE) | [`esp-hal`](https://docs.espressif.com/projects/rust/), with Espressif's compiler fork for Xtensa | **No** | Low to medium |
| **Raspberry Pi Zero 2 W** | 2.4 GHz Wi-Fi (802.11 b/g/n) and Bluetooth 4.2 (BLE) | A Linux computer: standard Rust (`aarch64-unknown-linux-gnu`), community crate [`rppal`](https://docs.rs/rppal) for its pins | **No:** Rullst has not run on any Raspberry Pi | Low for the board; medium with the microSD card, power supply and cables it needs |

### BBC micro:bit V2

The micro:bit V2 (nRF52833: Arm Cortex-M4 with FPU at 64 MHz, 512 KB flash,
128 KB RAM) has an accelerometer and a magnetometer, light sensing through its
5×5 LED display, a temperature sensor inside the processor, a microphone, a
speaker, two buttons and a touch-sensitive logo. It talks to other micro:bits
over its own 2.4 GHz radio protocol, which the micro:bit technical site
describes as unencrypted, and to other devices over Bluetooth Low Energy. It
has no Wi-Fi. To reach a server, a second micro:bit plugged into the computer
by USB can act as a bridge: it receives the radio messages and prints them to
its USB serial port, where a gateway program reads them. That bridge is a
design for your class, not something Rullst has tested. The micro:bit is very
beginner-friendly: MakeCode blocks or Python, and many robot kits built
around it (see the micro:bit Foundation's
[accessories](https://microbit.org/buy/accessories/) page and the robotics
extensions in the [MakeCode gallery](https://makecode.microbit.org/extensions/extension-gallery)).
In Rust, the community's `microbit-v2` crate and `embassy-nrf` support it, and
the Rust Embedded Working Group's
[Discovery book](https://docs.rust-embedded.org/discovery-mb2/) teaches with it.

**Validated with Rullst.** On 2026-11-10 a micro:bit V2 ran `rullst-iot`
itself (`default-features = false`, with the `microbit-v2` 0.16 crate): 66
checks of telemetry JSON, MQTT and CoAP encoding, the anomaly detector, signed
firmware verification (including an update signed by `cargo rullst iot:sign`)
and the rollback-counter contract, with no failure
([results](crates/iot.md#hardware-validation-bbc-microbit-v2)). The board
reported over USB serial; its radio, Bluetooth and the bridge above were not
part of that run.

### Arduino Uno R3

The Uno R3 has an ATmega328P at 16 MHz with 32 KB of flash and 2 KB of SRAM,
14 digital pins (6 with PWM) and 6 analog inputs. It has no radio and reaches
a computer through USB serial. It is usually programmed in C++ with the
Arduino IDE. In Rust, the community's `avr-hal` (`arduino-hal`) supports it
with a nightly compiler, and Rust's AVR target is Tier 3 (not built or tested
automatically by the Rust project). The Uno R4 WiFi is a different board, with
a Renesas RA4M1 and an ESP32-S3 module for Wi-Fi and Bluetooth, which Rullst
has not tested.

**Validated with Rullst, as a sensor board.** On 2026-11-10 an Uno's readings
went through `SensorTelemetry`, the MQTT encoder and `AnomalyDetector` in a
Rullst gateway on the computer. On 2026-11-11 two Uno-compatible boards (CH340
USB chip) streamed readings at the same time through one gateway each to
`rullst::iot_ingest`, and after one board was revoked every later post with
its token was refused
([two-board run](crates/iot.md#two-boards-through-rullstiot_ingest)). The
boards ran an Arduino sketch and `rullst-iot` ran on the computer: with 2 KB
of RAM, the Uno cannot hold the roughly 8 KiB of stack that `rullst-iot`'s
signature check needs. No broker or radio was involved.

### ESP32-C3

The ESP32-C3 has one 32-bit RISC-V core at up to 160 MHz, 400 KB of SRAM,
2.4 GHz Wi-Fi (802.11 b/g/n) and Bluetooth 5 (LE), on low-cost development
boards. Espressif backs its Rust support: `esp-hal` 1.0 (October 2025) was
announced as the first vendor-backed Rust SDK, with GPIO, UART, SPI and I2C in
the stable API and other drivers behind the `unstable` feature. Wi-Fi and
Bluetooth go through `esp-radio`, which had not reached 1.0 on 2026-11-19. The
C3 builds with stable Rust for `riscv32imc-unknown-none-elf`, and Espressif's
[no_std training](https://docs.espressif.com/projects/rust/no_std-training/)
is written for it. With Wi-Fi, a C3 could post `SensorTelemetry` JSON to
`TelemetryReceiver` with no gateway, using an HTTP client you choose; it would
also need the current time, because the receiver refuses timestamps more than
5 minutes ahead or 24 hours old by default.

**Not yet validated with `rullst-iot`.** Running the micro:bit's checks on an
ESP32-C3 is a 13.x candidate waiting for a board
([#637](https://github.com/Rullst/Rullst/issues/637)). CI compiles
`rullst-iot` for `riscv32imac-unknown-none-elf`, a related RISC-V target, but
not for the C3's own `riscv32imc-unknown-none-elf`, and compiling is not a
hardware test.

### Raspberry Pi Pico 2 W

The Pico 2 W has an RP2350 (two Arm Cortex-M33 or two Hazard3 RISC-V cores at
150 MHz, 520 KB of SRAM) with 4 MB of flash, and an Infineon CYW43439 for
2.4 GHz Wi-Fi (802.11n) and Bluetooth 5.2. Its 12 PIO state machines are small
programmable I/O processors. In Rust, the community's `embassy-rp` supports
the RP2040 and the RP235x and ships PIO programs for PWM, quadrature encoders
and stepper motors (`pio_programs`), which suit servos, wheel encoders and
steppers; `rp235x-hal` from the rp-rs project is the other HAL, and the
`cyw43` driver lists the Pico 2 W as working for Wi-Fi and Bluetooth. The Arm
cores use the `thumbv8m.main-none-eabihf` target.

**Not validated.** No board has run `rullst-iot`, and CI does not compile it
for `thumbv8m.main-none-eabihf`.

### ESP32-S3 and camera boards

The ESP32-S3 has two Xtensa LX7 cores at up to 240 MHz, 512 KB of SRAM,
2.4 GHz Wi-Fi (802.11 b/g/n), Bluetooth 5 (LE) and vector instructions for
neural-network and signal-processing work. Espressif's ESP32-S3-EYE board adds
a 2 MP camera, an LCD, a microphone, 8 MB of flash and 8 MB of PSRAM, for
simple vision exercises such as finding a colored marker. `esp-hal` supports
the S3, but Xtensa needs Espressif's fork of the Rust compiler, installed with
`espup`. For cameras, `esp-hal` has a DVP camera interface
(`lcd_cam::cam`) behind its `unstable` feature, and setting up the camera
sensor is your code; Espressif's camera driver,
[`esp32-camera`](https://github.com/espressif/esp32-camera), is a C component
for ESP-IDF. `TelemetryReceiver` accepts only `SensorTelemetry` readings, so
sending images to a Rullst server needs an upload route of your own.

**Not validated.**

### Raspberry Pi Zero 2 W

The Zero 2 W is a small Linux computer, not a microcontroller: a 1 GHz
quad-core 64-bit Arm Cortex-A53, 512 MB of RAM, 2.4 GHz Wi-Fi (802.11 b/g/n)
and Bluetooth 4.2 with BLE. It runs the 64-bit Raspberry Pi OS, so it could
run a Rullst server on the robot itself, with the web panel and the motor
control on one machine (untested, see below). `aarch64-unknown-linux-gnu` is a
Tier 1 Rust target, and `rppal` reaches its GPIO, I2C, PWM, SPI and UART.
Build on a laptop and copy the binary: 512 MB is little memory for compiling a
Rullst application. Linux does not guarantee timing the way a microcontroller
does, so precise pulses (servos, encoders) are often left to a microcontroller
board connected over serial.

**Not validated.** Rullst has not run on any Raspberry Pi. For scale only: the
Blank starter's release build used about 10 MB of resident memory when idle on
an x86_64 Linux CI runner
([budget file](https://github.com/Rullst/Rullst/blob/main/.github/starter-footprint/blank.toml));
nobody has measured it on ARM.

## Project ideas

These are ideas, not shipped examples. Each one names the Rullst parts it
would use (checked in the repository on 2026-11-19), what you build or bring,
what students learn, and a difficulty level.

### 1. Classroom weather station

- **Goal:** stations around the room, or outside a window, report
  temperature, light and sound level; a page shows live values and a chart of
  the day.
- **Boards:** micro:bit V2 stations (temperature, light and microphone are
  built in) with a USB-connected micro:bit as the radio bridge; or Uno boards
  with sensors over USB serial; or an ESP32-C3 or Pico 2 W posting over Wi-Fi.
- **Rullst parts:** `TelemetryReceiver` (feature `iot-ingest`) with one token
  per station and a `DeviceDirectory` table (copy the example's
  `src/registry.rs`); `BroadcastManager` and `RealtimeSse` to push readings to
  the page, as the example's dashboard does; an ORM model for the history
  (`cargo rullst make:model Reading --migration`, features `orm` and
  `strict-sqlite`), written by a task subscribed to the `iot.telemetry`
  channel; `AnomalyDetector` to mark readings outside a band (`normal`,
  `warning`, `critical anomaly`).
- **You build or bring:** the station and bridge firmware; the task that saves
  readings (the example keeps its history table in memory); the chart, since
  Rullst has no chart component: draw an SVG line in an `html!` view or add a
  chart library you choose.
- **Students learn:** sensors and units, sampling, calibration (compare the
  micro:bit's processor temperature with a thermometer), time series,
  databases, and how a page updates live.
- **Difficulty:** beginner to intermediate, starting from the example.

The roadmap records a related idea, an
[offline IoT example](../../ROADMAP.md#ideas-recorded-for-after-130) with
readings in SQLite and a dashboard without internet.

### 2. Robot with a web control panel

- **Goal:** drive a small robot from a phone or laptop browser (forward, back,
  left, right, stop) and watch its battery voltage and distance sensor live.
- **Boards:** Rullst on a laptop, with the motor board (an Uno, or a micro:bit
  in a robot kit) on a USB cable or behind the micro:bit radio bridge; or
  Rullst on a Pi Zero 2 W riding on the robot, talking to the motor board over
  serial or driving a motor driver through `rppal`; or an ESP32-C3 or Pico 2 W
  motor board on Wi-Fi.
- **Rullst parts:** an `html!` page whose buttons send `hx-post` requests:
  commands change the robot's state, so they are POST requests with the CSRF
  token ([CSRF form contract](tutorials/07-forms-and-validation.md#step-3-send-the-csrf-token-with-browser-forms)),
  never GET; `BroadcastManager` and `RealtimeSse` for the telemetry, and
  `TelemetryReceiver` if it arrives over HTTP; `cargo rullst auth` to put the
  panel behind a login. In development the server listens on `127.0.0.1`
  only; set `HOST=0.0.0.0` so phones on the classroom network can open it.
- **You build or bring:** the command path to the motor board, which Rullst
  does not provide (`iot_ingest` only receives readings). Over USB serial, the
  [`serialport`](https://crates.io/crates/serialport) crate that the example's
  gateway reads with can also write; it blocks, so give it its own thread or
  `spawn_blocking` instead of calling it in a request handler. Over Wi-Fi,
  the board needs its own HTTP or MQTT client, and MQTT also needs a broker
  (for example Eclipse Mosquitto) and a client crate such as
  [`rumqttc`](https://crates.io/crates/rumqttc) on the server. The motor
  firmware must stop when commands stop arriving (see
  [safety](#safety-notes-for-classes)).
- **Students learn:** client and server, why commands use POST, latency,
  serial protocols, motor drivers and PWM, and fail-safe design.
- **Difficulty:** intermediate; advanced on the Pi Zero 2 W, which needs
  cross-compilation.

### 3. Line-follower race scoreboard

- **Goal:** time each robot between a start gate and a finish gate and show a
  live class leaderboard on the projector and on phones.
- **Boards:** one micro:bit or Uno wired to both gate sensors (break-beam or
  reflective sensors), so one clock measures the whole lap; it reports the lap
  over USB serial or through the micro:bit radio bridge.
- **Rullst parts:** `TelemetryReceiver` receiving a `lap_ms` reading;
  `BroadcastManager` and `RealtimeSse` to update the leaderboard;
  `PresenceTracker` to show how many screens follow the race (one
  `user_joined` per open page, one `user_left` when it closes,
  `count_online`); an ORM model for the laps
  (`cargo rullst make:model Lap --migration`).
- **You build or bring:** the gate sensors and their firmware, a teacher page
  to choose which team is on the track, and the ranking rules.
  `SensorTelemetry` timestamps are whole seconds, so the board measures the
  lap and sends the milliseconds as the reading's value, rather than two
  timestamps.
- **Students learn:** clocks and timing (why one clock measures the lap),
  sensor debouncing, sorting, fair rules and realtime pages.
- **Difficulty:** intermediate.

The roadmap also records a
[multiplayer quiz idea](../../ROADMAP.md#ideas-recorded-for-after-130) built
on `BroadcastManager` and `PresenceTracker`.

### 4. Signed firmware updates

- **Goal:** show why a device checks a signature before installing firmware.
  The teacher signs an image, the board accepts it, then a copy with one
  changed byte, a changed version or another key is refused.
- **Boards:** micro:bit V2, the only board on which this has run.
- **Rullst parts:** `cargo rullst iot:keygen` (an Ed25519 key pair: the
  secret key with mode `0600` on Unix, the public key as `.pub.rs`, `.pub.bin`
  and `.pub`) and `cargo rullst iot:sign` (the manifest and its signature,
  checked with `OtaManager::begin_update` before they are written); on the
  board, `OtaManifest::from_signing_bytes`, `OtaManager::begin_update` and the
  `OtaImageVerifier` it returns, with a `RollbackCounterStore` for the
  anti-rollback counter. The repository's micro:bit test firmware,
  [`.github/fixtures/iot-microbit-v2`](https://github.com/Rullst/Rullst/tree/main/.github/fixtures/iot-microbit-v2),
  already runs this demonstration when built with `RULLST_IOT_SIGNED_DIR` set
  to the folder holding the files below, and did so on 2026-11-10
  ([results](crates/iot.md#cli-signed-update-on-the-board)). Its
  `firmware/src/main.rs` explains how to flash it and read the `PASS` and
  `FAIL` lines from the USB serial port.

  ```console
  cargo rullst iot:keygen --out ota.key
  cargo rullst iot:sign --key ota.key --image image.bin --target microbit-v2 \
    --version 2.0.0 --counter 7
  ```

- **You build or bring:** `image.bin`, any file standing in for a firmware
  image. `rullst-iot` only decides whether an image may be installed: it
  does not download, write flash or reboot, so in this demonstration nothing
  is actually installed. A real update path needs your own download,
  flashing and bootloader code.
- **Students learn:** hashes (SHA-256), secret and public keys, digital
  signatures, why a rollback counter stops an older, vulnerable version from
  coming back, and why the secret key never leaves the teacher's computer.
- **Difficulty:** advanced to set up; the demonstration itself suits
  intermediate students.

### 5. Programming exercises graded by `rullst-labs`

- **Goal:** students submit small functions, for example turning two
  line-sensor readings into a steering value, and see which test cases pass
  before they try the code on a robot.
- **Boards:** none for grading. The code runs on a computer; students copy it
  to the robot afterwards.
- **Rullst parts:** `rullst-labs`, a direct dependency (the facade has no
  feature for it). An `Exercise` holds up to 64 `GraderCase`s, each with two
  whole numbers in (`input: [i64; 2]`) and one expected out
  (`expected: i64`), which fits steering from two sensor readings, speed from
  distance and time, or stop-or-go from a distance and a threshold.
  `SqliteLabs` (feature `sqlite`) keeps submissions encrypted, grades them
  exactly against the stored answers and gives feedback that never reveals
  the hidden inputs or expected values. The
  [`course_app`](https://github.com/Rullst/Rullst/blob/main/rullst-labs/examples/course_app.rs)
  example shows the application side.
- **The host must provide:** `rullst-labs` never runs students' code; you
  bring your own runner
  ([contract](labs-runner-contract.md#verified-by-rullst-labs-versus-guaranteed-by-the-host)).
  That is a trusted controller (features `sqlite` and `receipt-signing`, with
  its own Ed25519 signing seed) that claims jobs and signs results, and an
  isolated worker that compiles or runs each submission with no network, no
  access to the job database or keys, and CPU, memory and time limits
  ([isolation](labs-runner-contract.md#isolation-the-host-owns)). The
  application supplies signed-in students and their course enrollment. The
  source arrives as text (`RustSource`). Protocol v1 defines only one
  profile, a Rust function (`rust-function-wasm-v1`); a runner that compiles
  or interprets another language does so outside that profile. In the
  13.0.0-alpha.5 acceptance run a runner that started one Docker container per
  job graded Python submissions that way, and the contract states that a
  container alone is not enough isolation for hostile code.
- **Students learn:** writing a function to a specification, test cases and
  edge cases (zero, negative readings), and why hidden tests exist.
- **Difficulty:** beginner for students; advanced for the teacher, who sets
  up and isolates the runner.

## Safety notes for classes

- **Power motors separately.** Drive motors through a motor driver or a robot
  kit's driver board from their own battery, never straight from a board's
  pin, and connect the grounds of the motor supply and the board. Run the
  first tests with the wheels off the table.
- **Fail safe.** Motor firmware stops when commands stop arriving (for
  example after half a second without one), and the panel has a stop button.
- **Batteries.** Charge each battery with a charger made for its type, do not
  leave lithium batteries charging unattended, retire a swollen, damaged or
  hot battery, and cover the terminals of stored batteries.
- **No secrets in student firmware.** Device tokens belong to the gateway's
  environment (`IOT_DEVICE_TOKEN` in the example), not in sketches or in a
  repository; give each board its own token and revoke a lost one. Keep
  boards on a classroom Wi-Fi network separate from the school's, since its
  password ends up in firmware. The firmware-signing secret key stays on the
  teacher's computer; boards hold only the public key. The micro:bit radio is
  unencrypted, so treat what arrives over it as untrusted.

## Where to go next

- [rullst-iot guide](crates/iot.md): what each module does, the hardware
  results, signing with the CLI and receiving telemetry.
- [Tutorial 29: IoT data and frame helpers](tutorials/29-embedded-iot-edge.md).
- [`examples/iot-gateway`](https://github.com/Rullst/Rullst/tree/main/examples/iot-gateway):
  the validated Uno boards, gateway, registry and dashboard.
- [Tutorial 34: realtime dashboard with Server-Sent Events](tutorials/34-realtime-dashboard-sse.md).
- [Labs runner contract](labs-runner-contract.md).
- The roadmap's [ideas for after 13.0](../../ROADMAP.md#ideas-recorded-for-after-130),
  including the offline IoT example.
- [Issue #637](https://github.com/Rullst/Rullst/issues/637): validating
  `rullst-iot` on an ESP32-C3. A class with a board can help.

## Sources for the board facts

Checked on 2026-11-19.

- micro:bit V2: [features](https://microbit.org/get-started/features/overview/)
  and [V2.2x hardware](https://tech.microbit.org/hardware/2-2-revision/)
  (micro:bit Foundation); [`microbit-v2`](https://docs.rs/microbit-v2),
  [nrf-rs/microbit](https://github.com/nrf-rs/microbit) and
  [`embassy-nrf`](https://docs.rs/embassy-nrf).
- Arduino Uno R3: [product page](https://docs.arduino.cc/hardware/uno-rev3/)
  and [datasheet](https://docs.arduino.cc/resources/datasheets/A000066-datasheet.pdf);
  [Uno R4 WiFi](https://docs.arduino.cc/hardware/uno-r4-wifi/);
  [`avr-hal`](https://github.com/Rahix/avr-hal).
- ESP32-C3 and ESP32-S3: Espressif's
  [ESP32-C3](https://www.espressif.com/en/products/socs/esp32-c3) and
  [ESP32-S3](https://www.espressif.com/en/products/socs/esp32-s3) pages, the
  [ESP32-C3 datasheet](https://documentation.espressif.com/esp32-c3_datasheet_en.pdf),
  [ESP32-S3-EYE](https://www.espressif.com/en/products/devkits/esp32-s3-eye/overview),
  the [esp-hal 1.0 announcement](https://developer.espressif.com/blog/2025/10/esp-hal-1/),
  the [toolchain chapter](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
  of Espressif's Rust book, [`esp-hal`](https://github.com/esp-rs/esp-hal) and
  [`esp-radio`](https://crates.io/crates/esp-radio).
- Raspberry Pi: [Pico 2](https://www.raspberrypi.com/products/raspberry-pi-pico-2/),
  [Pico-series documentation](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html),
  [Zero 2 W](https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/),
  [operating systems](https://www.raspberrypi.com/software/operating-systems/);
  [`embassy-rp`](https://docs.rs/embassy-rp), [`rp235x-hal`](https://docs.rs/rp235x-hal),
  [`cyw43`](https://docs.rs/cyw43) and [`rppal`](https://docs.rs/rppal).
- Rust targets: [platform support](https://doc.rust-lang.org/rustc/platform-support.html).
