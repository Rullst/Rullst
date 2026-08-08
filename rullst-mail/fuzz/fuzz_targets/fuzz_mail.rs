#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_mail::Message;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let msg = Message::new().to(s).subject("Test").html(s).text(s);
        assert_eq!(msg.to, s);
    }
});
