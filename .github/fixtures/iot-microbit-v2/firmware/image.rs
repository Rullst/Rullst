// The synthetic firmware image the OTA checks verify: 32 KiB from a fixed
// xorshift32 seed. `build.rs` writes it for the firmware and the vectors
// generator signs the same bytes, so neither checks in a binary file.

/// Length of the synthetic image in bytes.
pub const FIRMWARE_LEN: usize = 32 * 1024;

/// The deterministic image bytes.
pub fn firmware_image() -> Vec<u8> {
    let mut state: u32 = 0x1234_5678;
    (0..FIRMWARE_LEN)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 24) as u8
        })
        .collect()
}
