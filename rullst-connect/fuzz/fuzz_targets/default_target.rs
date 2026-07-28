#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_connect::extractors::AuthCallback;

fuzz_target!(|data: &[u8]| {
    // Alvo real de Fuzzing:
    // O objetivo é tentar causar um "panic!" (crash) no desserializador
    // de URLs alimentando-o com dados corrompidos. 
    // Em OAuth2, as URLs de callback são vetores de ataque comuns.
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_urlencoded::from_str::<AuthCallback>(s);
    }
});
