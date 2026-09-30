//! `make:migration` writes this template into the application crate, where
//! `crate::` names the application rather than `rullst_orm`. Including it
//! from an integration test compiles it outside the ORM crate as well.

mod generated {
    include!("../src/migration_template.rs.txt");
}

use rullst_orm::schema::Migration;

#[test]
fn generated_migration_template_compiles_in_an_application_crate() {
    assert_eq!(generated::MigrationImpl.name(), "m{timestamp}_{name}");
}
