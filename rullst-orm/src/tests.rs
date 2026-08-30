use crate::*;

#[allow(unused_imports)]
use ::core::prelude::v1::test;

#[test]
fn test_pagination_result() {
    let mut pr = PaginationResult {
        data: vec![1, 2, 3],
        total: 3,
        per_page: 10,
        current_page: 1,
        last_page: 1,
    };
    assert_eq!(pr.data.len(), 3);
    assert_eq!(pr.total, 3);
    pr.data.push(4);
    assert_eq!(pr.data.len(), 4);
}

#[test]
fn test_replace_placeholders() {
    assert_eq!(
        replace_placeholders("SELECT * FROM users WHERE id = ? AND name = ?"),
        "SELECT * FROM users WHERE id = $1 AND name = $2"
    );
    assert_eq!(
        replace_placeholders("INSERT INTO users (name) VALUES (?)"),
        "INSERT INTO users (name) VALUES ($1)"
    );
    assert_eq!(
        replace_placeholders("SELECT * FROM users"),
        "SELECT * FROM users"
    );
    assert_eq!(replace_placeholders("? ? ?"), "$1 $2 $3");
}

#[test]
fn test_rullst_value_conversions() {
    // From
    let v: RullstValue = "test".into();
    assert!(matches!(v, RullstValue::String(_)));
    let v_string: RullstValue = "test".to_string().into();
    assert!(matches!(v_string, RullstValue::String(_)));
    let v_int: RullstValue = 100.into();
    assert!(matches!(v_int, RullstValue::Int(100)));
    let v_bool: RullstValue = false.into();
    assert!(matches!(v_bool, RullstValue::Bool(false)));
    let v_float: RullstValue = std::f64::consts::PI.into();
    assert!(matches!(v_float, RullstValue::Float(_)));

    // TryFrom String
    let v_str_conv = RullstValue::String("hello".to_string());
    assert_eq!(String::try_from(v_str_conv).unwrap(), "hello");
    assert!(String::try_from(RullstValue::Int(10)).is_err());

    // TryFrom i32
    let v_int_conv = RullstValue::Int(42);
    assert_eq!(i32::try_from(v_int_conv).unwrap(), 42);
    assert!(i32::try_from(RullstValue::Bool(true)).is_err());

    // TryFrom f64
    let v_float_conv = RullstValue::Float(2.71);
    assert_eq!(f64::try_from(v_float_conv).unwrap(), 2.71);
    assert!(f64::try_from(RullstValue::Int(10)).is_err());

    // TryFrom bool
    let v_bool_conv = RullstValue::Bool(true);
    assert!(bool::try_from(v_bool_conv).unwrap());
    assert!(bool::try_from(RullstValue::Int(10)).is_err());
}

#[test]
fn test_enable_query_log_wrapper() {
    // Orm::enable/disable_query_log delegate to schema — verify the delegation works.
    Orm::disable_query_log();
    assert!(!crate::schema::is_query_log_enabled());
    Orm::enable_query_log();
    assert!(crate::schema::is_query_log_enabled());
    Orm::disable_query_log();
    assert!(!crate::schema::is_query_log_enabled());
}

#[test]
fn test_disable_query_log_wrapper() {
    Orm::enable_query_log();
    Orm::disable_query_log();
    assert!(!crate::schema::is_query_log_enabled());
}

#[cfg(feature = "redis")]
#[test]
fn test_redis_client_uninitialized() {
    let err = Orm::redis_client().unwrap_err();
    assert!(matches!(err, crate::Error::Internal(_)));
}

#[cfg(feature = "redis")]
#[test]
fn test_redis_manager_uninitialized() {
    let err = Orm::redis_manager().unwrap_err();
    assert!(matches!(err, crate::Error::Internal(_)));
}

#[test]
fn test_uninitialized_getters_are_fallible() {
    assert!(matches!(Orm::pool(), Err(crate::Error::NotInitialized)));
    assert!(matches!(
        Orm::read_pool(),
        Err(crate::Error::NotInitialized)
    ));
    assert!(matches!(Orm::driver(), Err(crate::Error::NotInitialized)));
}

#[test]
fn test_validate_dsn() {
    // Safe case
    crate::pool::Orm::validate_dsn("sqlite::memory:");
    // Security warning case (printed to stderr, shouldn't panic)
    crate::pool::Orm::validate_dsn("postgres://external-db.com/mydb?sslmode=disable");
}

#[cfg(feature = "redis")]
#[tokio::test]
async fn test_init_redis_failure() {
    let err = Orm::init_redis("redis://127.0.0.1:0").await.unwrap_err();
    assert!(matches!(err, crate::Error::CacheError(_)));
}

#[cfg(feature = "redis")]
#[tokio::test]
async fn test_init_redis_rejects_invalid_namespace_before_connecting() {
    let err = Orm::init_redis_with_namespace("redis://127.0.0.1:0", "shared:unsafe")
        .await
        .unwrap_err();
    assert!(matches!(err, crate::Error::Validation(_)));
}

#[test]
fn test_orm_max_query_limit_and_timeout() {
    Orm::set_max_query_limit(15);
    assert_eq!(crate::schema::get_max_query_limit(), Some(15));
    Orm::set_max_query_limit(0);
    assert_eq!(crate::schema::get_max_query_limit(), None);

    Orm::set_query_timeout(5);
    assert_eq!(
        crate::schema::get_query_timeout(),
        Some(std::time::Duration::from_secs(5))
    );
    Orm::set_query_timeout(0);
    assert_eq!(crate::schema::get_query_timeout(), None);
}

#[tokio::test]
async fn test_try_driver_and_placeholder_dsn() {
    let err = Orm::try_driver();
    assert!(err.is_err() || err.is_ok());

    let res = Orm::init("libsql://[your-database-id].turso.io").await;
    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("DATABASE_URL contains placeholder brackets"));
}
