pub(super) const FOUNDATION_MIGRATIONS_MODULE: &str = r##"pub mod m20260601000000_create_lms_tables;
pub mod m20260827000000_add_learning_access;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_lms_tables::MigrationImpl),
        Box::new(m20260827000000_add_learning_access::MigrationImpl),
    ]
}
"##;
