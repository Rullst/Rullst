#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz SQL identifier validation
        let _ = rullst_orm::schema::validate_identifier(s);
        
        // Fuzz table name validation
        let _ = rullst_orm::schema::validate_table_name(s);
        
        // Fuzz JoinClause::on validation (first argument)
        let mut jc = rullst_orm::schema::JoinClause::new("users");
        jc.on(s, "=", "id");
        
        // Fuzz JoinClause::on validation (second argument)
        jc.on("id", "=", s);
        
        let _sql = jc.to_sql();
    }
});
