use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn type_codecs(name: &syn::Ident, type_name: &str) -> TokenStream {
    #[cfg(feature = "runtime-driver-codecs")]
    {
        quote! {
            rullst_orm::__rullst_enum_sqlite!(#name);
            rullst_orm::__rullst_enum_mysql!(#name);
            rullst_orm::__rullst_enum_postgres!(#name, #type_name);
        }
    }
    #[cfg(not(feature = "runtime-driver-codecs"))]
    {
        quote! {
        impl rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Sqlite> for #name {
            fn type_info() -> rullst_orm::_sqlx::sqlite::SqliteTypeInfo {
                <str as rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Sqlite>>::type_info()
            }
        }

        impl rullst_orm::_sqlx::Type<rullst_orm::_sqlx::MySql> for #name {
            fn type_info() -> rullst_orm::_sqlx::mysql::MySqlTypeInfo {
                rullst_orm::_sqlx::mysql::MySqlTypeInfo::__enum()
            }

            fn compatible(type_info: &rullst_orm::_sqlx::mysql::MySqlTypeInfo) -> bool {
                <str as rullst_orm::_sqlx::Type<rullst_orm::_sqlx::MySql>>::compatible(type_info)
            }
        }

        impl rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Postgres> for #name {
            fn type_info() -> rullst_orm::_sqlx::postgres::PgTypeInfo {
                rullst_orm::_sqlx::postgres::PgTypeInfo::with_name(#type_name)
            }
        }

        impl rullst_orm::_sqlx::postgres::PgHasArrayType for #name {
            fn array_type_info() -> rullst_orm::_sqlx::postgres::PgTypeInfo {
                rullst_orm::_sqlx::postgres::PgTypeInfo::array_of(#type_name)
            }
        }
        }
    }
}
