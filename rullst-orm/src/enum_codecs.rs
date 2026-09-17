//! Driver-owned expansion points: cfg is evaluated here, never in the application.

#[cfg(any(feature = "drivers-all", feature = "strict-sqlite", feature = "turso"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __rullst_enum_sqlite {
    ($name:ident) => {
        impl $crate::_sqlx::Type<$crate::_sqlx::Sqlite> for $name {
            fn type_info() -> $crate::_sqlx::sqlite::SqliteTypeInfo {
                <str as $crate::_sqlx::Type<$crate::_sqlx::Sqlite>>::type_info()
            }
        }
    };
}

#[cfg(not(any(feature = "drivers-all", feature = "strict-sqlite", feature = "turso")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __rullst_enum_sqlite {
    ($name:ident) => {};
}

#[cfg(any(feature = "drivers-all", feature = "strict-mysql"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __rullst_enum_mysql {
    ($name:ident) => {
        impl $crate::_sqlx::Type<$crate::_sqlx::MySql> for $name {
            fn type_info() -> $crate::_sqlx::mysql::MySqlTypeInfo {
                $crate::_sqlx::mysql::MySqlTypeInfo::__enum()
            }
            fn compatible(type_info: &$crate::_sqlx::mysql::MySqlTypeInfo) -> bool {
                <str as $crate::_sqlx::Type<$crate::_sqlx::MySql>>::compatible(type_info)
            }
        }
    };
}

#[cfg(not(any(feature = "drivers-all", feature = "strict-mysql")))]
#[doc(hidden)]
#[macro_export]
macro_rules! __rullst_enum_mysql {
    ($name:ident) => {};
}

#[cfg(any(
    feature = "drivers-all",
    feature = "strict-postgres",
    feature = "pgvector"
))]
#[doc(hidden)]
#[macro_export]
macro_rules! __rullst_enum_postgres {
    ($name:ident, $type_name:literal) => {
        impl $crate::_sqlx::Type<$crate::_sqlx::Postgres> for $name {
            fn type_info() -> $crate::_sqlx::postgres::PgTypeInfo {
                $crate::_sqlx::postgres::PgTypeInfo::with_name($type_name)
            }
        }
        impl $crate::_sqlx::postgres::PgHasArrayType for $name {
            fn array_type_info() -> $crate::_sqlx::postgres::PgTypeInfo {
                $crate::_sqlx::postgres::PgTypeInfo::array_of($type_name)
            }
        }
    };
}

#[cfg(not(any(
    feature = "drivers-all",
    feature = "strict-postgres",
    feature = "pgvector"
)))]
#[doc(hidden)]
#[macro_export]
macro_rules! __rullst_enum_postgres {
    ($name:ident, $type_name:literal) => {};
}
