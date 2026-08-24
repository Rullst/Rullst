use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_update_builder(parsed: &ParsedModel) -> (TokenStream, TokenStream) {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let update_builder_name = quote::format_ident!("{}UpdateBuilder", name);
    let normal_fields = &parsed.normal_fields;
    let normal_fields_types = &parsed.normal_fields_types;

    let mut builder_fields = vec![];
    let mut builder_methods = vec![];
    let mut set_clauses = vec![];
    let mut update_bindings = vec![];
    let mut apply_to_model = vec![];
    let mut builder_inits = vec![];

    for (field, ty) in normal_fields.iter().zip(normal_fields_types.iter()) {
        if field == "id" {
            continue;
        }

        builder_fields.push(quote! {
            #field: Option<#ty>
        });

        builder_inits.push(quote! {
            #field: None
        });

        builder_methods.push(quote! {
            pub fn #field(mut self, value: #ty) -> Self {
                self.#field = Some(value);
                self
            }
        });

        let field_str = field.to_string();
        set_clauses.push(quote! {
            if self.#field.is_some() {
                sets.push(format!("{} = ?", #field_str));
            }
        });

        update_bindings.push(quote! {
            if let Some(ref val) = self.#field {
                exec = exec.bind(val.clone());
            }
        });

        apply_to_model.push(quote! {
            if let Some(ref val) = self.#field {
                self.model.#field = val.clone();
            }
        });
    }

    let struct_def = quote! {
        pub struct #update_builder_name<'a> {
            model: &'a mut #name,
            #(#builder_fields),*
        }

        impl<'a> #update_builder_name<'a> {
            #(#builder_methods)*

            pub async fn save(mut self) -> Result<(), rullst_orm::Error> {
                let mut sets = vec![];
                #(#set_clauses)*

                if sets.is_empty() {
                    return Ok(()); // Nothing to update
                }

                #(#apply_to_model)*

                let driver = rullst_orm::Orm::driver()?;
                let mut sql = format!("UPDATE {} SET {} WHERE id = ?", #table_name, sets.join(", "));
                if driver == "postgres" {
                    sql = rullst_orm::replace_placeholders(&sql);
                }

                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug Partial Update] {:?} | ID: {}", sql, self.model.id);
                }

                let pool = rullst_orm::Orm::try_pool()?;
                let query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
                let mut exec = query;

                #(#update_bindings)*

                exec = exec.bind(self.model.id);
                rullst_orm::execute_query!(exec, execute, pool)?;

                Ok(())
            }
        }
    };

    let method_def = quote! {
        pub fn update_partial(&mut self) -> #update_builder_name<'_> {
            #update_builder_name {
                model: self,
                #(#builder_inits),*
            }
        }
    };

    (struct_def, method_def)
}
