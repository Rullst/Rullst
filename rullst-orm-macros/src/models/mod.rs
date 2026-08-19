pub mod ai_ops;
pub mod column_enum;
pub mod crud_ops;
pub mod json_ops;
pub mod query_ops;
pub mod redis_ops;
pub mod update_builder;

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub use ai_ops::generate_ai_methods;
pub use column_enum::generate_column_enum;
pub use crud_ops::{generate_delete_methods, generate_save_method};
pub use json_ops::generate_json_methods;
pub use query_ops::{generate_query_methods, generate_search_method};
pub use redis_ops::generate_redis_hash_methods;
pub use update_builder::generate_update_builder;

pub fn generate(parsed: &ParsedModel, relationship_methods: &[TokenStream]) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let builder_name = quote::format_ident!("{}QueryBuilder", name);
    let observer_trait_name = quote::format_ident!("{}Observer", name);

    let enum_def = generate_column_enum(parsed);
    let json_methods = generate_json_methods(parsed);
    let search_method = generate_search_method(parsed, &builder_name);
    let save_method = generate_save_method(parsed);
    let delete_methods = generate_delete_methods(parsed);
    let query_methods = generate_query_methods(parsed, &builder_name);
    let (update_builder_struct, update_builder_method) = generate_update_builder(parsed);
    let redis_methods = generate_redis_hash_methods(parsed);
    let ai_methods = generate_ai_methods(parsed);

    quote! {
        #enum_def
        #update_builder_struct

        #ai_methods

        #[rullst_orm::async_trait]
        impl rullst_orm::RullstModel for #name {
            fn table_name() -> &'static str {
                #table_name
            }
        }

        impl #name {
            #(#relationship_methods)*

            #json_methods

            pub fn observe(observer: std::sync::Arc<dyn #observer_trait_name + Send + Sync>) {
                let list = Self::observers();
                let mut writer = list.write().unwrap_or_else(|poisoned| poisoned.into_inner());
                writer.push(observer);
            }

            fn observers() -> &'static std::sync::RwLock<Vec<std::sync::Arc<dyn #observer_trait_name + Send + Sync>>> {
                static LIST: std::sync::OnceLock<std::sync::RwLock<Vec<std::sync::Arc<dyn #observer_trait_name + Send + Sync>>>> = std::sync::OnceLock::new();
                LIST.get_or_init(|| std::sync::RwLock::new(vec![]))
            }

            #search_method
            #query_methods

            #update_builder_method

            #save_method
            #delete_methods

            #redis_methods
        }
    }
}
