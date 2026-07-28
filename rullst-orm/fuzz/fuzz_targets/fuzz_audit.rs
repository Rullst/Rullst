#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Tenta converter os bytes aleatórios para string
    if let Ok(s) = std::str::from_utf8(data) {
        // Divide a string na metade de forma segura (respeitando bordas de caracteres UTF-8)
        let mid = s.floor_char_boundary(s.len() / 2);
        let (old_json, new_json) = s.split_at(mid);
        
        // Fuzzing a função de diff de auditoria
        // O objetivo é garantir que essa função nunca cause um 'panic!'
        // mesmo com JSONs inválidos, cortados pela metade ou com caracteres obscuros.
        let _ = rullst_orm::audit::compute_diff(old_json, new_json);
    }
});
