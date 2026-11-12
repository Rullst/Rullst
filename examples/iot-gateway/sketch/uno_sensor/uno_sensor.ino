// Rullst IoT gateway example: Arduino Uno (ATmega328P) sensor node.
//
// Prints one line per second at 115200 baud:
//   dev=uno-a seq=12 temp_raw=333 temp_c=7.1 sine=0.59 xor=3F
//
// dev      : DEVICE_ID, set per board when compiling, for example
//            arduino-cli compile --fqbn arduino:avr:uno \
//              --build-property 'compiler.cpp.extra_flags=-DDEVICE_ID="uno-b"'
// seq      : sequence counter, restarts at 0 on every reset (opening the
//            serial port resets an Uno through DTR).
// temp_raw : ATmega328P internal temperature sensor, ADC channel 8 against the
//            internal 1.1 V reference, average of 8 conversions (first
//            conversion after switching the mux/reference is discarded).
// temp_c   : UNCALIBRATED, only roughly degrees Celsius. Uses the common
//            empirical approximation (raw - 324.31) / 1.22; the datasheet
//            gives about +/-10 C of part-to-part offset, so a real deployment
//            calibrates the offset per chip. It also measures the die, which
//            runs a little warmer than the room.
// sine     : synthetic sin(2*pi*seq/60), two decimals.
// xor      : XOR of every byte before " xor=", two upper-case hex digits, so
//            the gateway can reject a garbled line.
//
// Lines starting with '#' are informational and carry no reading.
//
// Test hook: receiving the byte 'g' makes the next cycle also emit five
// deliberately malformed lines (bad checksum, truncated, binary junk,
// over-long, out-of-range with a valid checksum) so the gateway's rejection
// path can be exercised on the real serial link.

#include <math.h>

#ifndef DEVICE_ID
#define DEVICE_ID "uno-a"
#endif
#define FIRMWARE_VERSION "2"

static const unsigned long PERIOD_MS = 1000UL;
static unsigned long seq = 0;
static unsigned long next_due = 0;
static bool garble_pending = false;

static unsigned int readInternalTemperatureRaw() {
  // REFS1|REFS0 = internal 1.1 V, MUX3 = channel 8 (temperature sensor).
  ADMUX = _BV(REFS1) | _BV(REFS0) | _BV(MUX3);
  ADCSRA |= _BV(ADEN);
  delay(5);  // let the reference and the mux settle
  ADCSRA |= _BV(ADSC);  // discard the first conversion
  while (bit_is_set(ADCSRA, ADSC)) {
  }
  unsigned long sum = 0;
  for (byte i = 0; i < 8; i++) {
    ADCSRA |= _BV(ADSC);
    while (bit_is_set(ADCSRA, ADSC)) {
    }
    sum += ADCW;
  }
  return (unsigned int)(sum / 8);
}

static byte xorOf(const char *text) {
  byte x = 0;
  while (*text) {
    x ^= (byte)*text++;
  }
  return x;
}

void setup() {
  Serial.begin(115200);
  Serial.print(F("# rullst-uno-sensor fw=" FIRMWARE_VERSION " dev="));
  Serial.println(DEVICE_ID);
  next_due = millis();
}

static void emitGarbage(const char *good_line) {
  // 1. A digit flipped after the checksum was computed.
  char bad[100];
  strncpy(bad, good_line, sizeof(bad) - 1);
  bad[sizeof(bad) - 1] = 0;
  // Skip "dev=..." so the flipped digit lands in a reading field.
  for (char *c = strchr(bad, ' '); c && *c; c++) {
    if (*c >= '0' && *c <= '8') { *c = *c + 1; break; }
  }
  char check[4];
  snprintf(check, sizeof(check), "%02X", xorOf(good_line));
  Serial.print(bad);
  Serial.print(F(" xor="));
  Serial.println(check);
  // 2. Truncated line.
  Serial.println(F("seq=99 temp_raw=3"));
  // 3. Binary junk.
  const byte junk[] = {0xFF, 0xFE, 0x00, 0x80, 's', 'e', 'q', '=', 0xC3, '\r', '\n'};
  Serial.write(junk, sizeof(junk));
  // 4. Over-long line.
  for (byte i = 0; i < 120; i++) {
    Serial.write('A');
  }
  Serial.println();
  // 5. Out of range but correctly checksummed.
  const char *range = "dev=" DEVICE_ID " seq=5 temp_raw=4000 temp_c=1.0 sine=0.00";
  snprintf(check, sizeof(check), "%02X", xorOf(range));
  Serial.print(range);
  Serial.print(F(" xor="));
  Serial.println(check);
}

void loop() {
  while (Serial.available() > 0) {
    if (Serial.read() == 'g') {
      garble_pending = true;
    }
  }
  unsigned long now = millis();
  if ((long)(now - next_due) < 0) {
    return;
  }
  next_due += PERIOD_MS;

  unsigned int raw = readInternalTemperatureRaw();
  float temp_c = ((float)raw - 324.31f) / 1.22f;
  float sine = sin(2.0f * (float)M_PI * (float)(seq % 60UL) / 60.0f);

  char temp_text[12];
  char sine_text[12];
  dtostrf(temp_c, 0, 1, temp_text);
  dtostrf(sine, 0, 2, sine_text);

  char line[100];
  snprintf(line, sizeof(line), "dev=%s seq=%lu temp_raw=%u temp_c=%s sine=%s", DEVICE_ID, seq,
           raw, temp_text, sine_text);
  char check[4];
  snprintf(check, sizeof(check), "%02X", xorOf(line));

  Serial.print(line);
  Serial.print(F(" xor="));
  Serial.println(check);
  if (garble_pending) {
    garble_pending = false;
    emitGarbage(line);
  }
  seq++;
}
