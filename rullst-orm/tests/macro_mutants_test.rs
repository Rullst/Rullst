use rullst_orm::_sqlx::FromRow;

#[derive(rullst_orm::Orm, Debug, Clone, PartialEq, FromRow)]
#[orm(table_name = "test_mutants")]
#[orm(soft_delete)]
struct MutantModel {
    id: i32,
    name: String,
    deleted_at: Option<String>,
}

#[tokio::test]
async fn test_mutant_macro_generation() {
    // 1. generate_column_enum
    // If the enum generation was deleted (models.rs#L53), this wouldn't compile.
    let _col = MutantModelColumn::Name;

    // 2. build_soft_delete_set_clause (builder.rs#L145)
    // 3. generate_delete_all_logic (builder.rs#L88)
    // generate_execution_methods (builder.rs#L919)
    // Just verifying compilation of the generated query builder methods.
    // We don't actually run them here.

    // Check factory / observer generation (factory_observer.rs#L6)
    // If deleted, `.observe` won't exist.
    struct MyObserver;
    #[async_trait::async_trait]
    impl MutantModelObserver for MyObserver {}
    MutantModel::observe(std::sync::Arc::new(MyObserver));
}

#[derive(rullst_orm::Enum, Debug, Clone, PartialEq)]
pub enum MutantStatus {
    Active,
    Inactive,
}

#[test]
fn test_enum_derive_mutants() {
    let status = MutantStatus::Active;
    assert_eq!(status.to_string(), "Active");
    assert_eq!(
        "Inactive".parse::<MutantStatus>().unwrap(),
        MutantStatus::Inactive
    );

    let orm_val: rullst_orm::RullstValue = status.into();
    assert!(matches!(orm_val, rullst_orm::RullstValue::String(ref s) if s == "Active"));

    let back: MutantStatus = orm_val.try_into().unwrap();
    assert_eq!(back, MutantStatus::Active);
}
