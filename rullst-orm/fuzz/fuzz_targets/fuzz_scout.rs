#![no_main]
use libfuzzer_sys::fuzz_target;
use rullst_orm::Orm;
use std::sync::Once;

#[derive(Debug, Clone, rullst_orm::FromRow, Orm)]
#[orm(searchable)]
pub struct ScoutUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

static INIT: Once = Once::new();

fuzz_target!(|data: &[u8]| {
    INIT.call_once(|| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let _ = rullst_orm::Orm::init("sqlite::memory:").await;
        });
    });

    if let Ok(s) = std::str::from_utf8(data) {
        // Just parsing the string and calling search. It should safely return an Error
        // since no search engine is actually configured in the fuzzer.
        let _ = ScoutUser::search(s);
    }
});
