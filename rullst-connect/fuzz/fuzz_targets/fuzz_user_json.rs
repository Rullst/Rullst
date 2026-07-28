#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_connect::user::ConnectUser;

fuzz_target!(|data: &[u8]| {
    // Alvo: Testar a robustez do parser JSON para perfis de usuário OAuth.
    // Provedores diferentes podem retornar tipos de dados imprevistos ou campos
    // ausentes. Isso garante que a biblioteca retorne um erro amigável (Err)
    // ao invés de derrubar o servidor em pânico.
    
    // LibFuzzer/ASan runs out of memory (RSS limit) when fuzzing highly nested JSON 
    // into serde_json::Value due to the overhead of tracking thousands of small allocations.
    // We restrict the depth here to prevent the fuzzer from timing out or hitting OOM.
    let mut depth: usize = 0;
    for &b in data {
        if b == b'{' || b == b'[' {
            depth += 1;
            if depth > 32 {
                return;
            }
        } else if b == b'}' || b == b']' {
            depth = depth.saturating_sub(1);
        }
    }

    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<ConnectUser>(s);
    }
});
