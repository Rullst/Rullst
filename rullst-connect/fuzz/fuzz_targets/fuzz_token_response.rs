#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_connect::provider::Oauth2TokenResponse;

fuzz_target!(|data: &[u8]| {
    // Alvo: Testar a desserialização da resposta de tokens.
    // A extração do access_token, refresh_token e id_token é crítica.
    // Fuzzing aqui previne ataques onde o provedor ou um atacante (Man-in-the-Middle)
    // adultere o JSON de resposta durante o token exchange.
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<Oauth2TokenResponse>(s);
    }
});
