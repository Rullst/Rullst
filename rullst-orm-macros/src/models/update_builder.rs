use crate::parser::{EncryptedFieldKind, ParsedModel};
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_update_builder(parsed: &ParsedModel) -> (TokenStream, TokenStream) {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let update_builder_name = quote::format_ident!("{}UpdateBuilder", name);
    let normal_fields = &parsed.normal_fields;
    let normal_fields_types = &parsed.normal_fields_types;
    let tenant_column = parsed.tenant_column.as_str();
    let tenant_field_type = normal_fields
        .iter()
        .zip(normal_fields_types.iter())
        .find(|(field, _)| *field == tenant_column)
        .map(|(_, ty)| ty);

    let mut builder_fields = vec![];
    let mut builder_methods = vec![];
    let mut set_clauses = vec![];
    let mut update_bindings = vec![];
    let mut apply_to_model = vec![];
    let mut builder_inits = vec![];

    for (field, ty) in normal_fields.iter().zip(normal_fields_types.iter()) {
        if field == "id" || (!tenant_column.is_empty() && field == tenant_column) {
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

        let encrypted_kind = parsed
            .encrypted_fields
            .iter()
            .find(|encrypted| encrypted.name == *field)
            .map(|encrypted| encrypted.kind);
        update_bindings.push(match encrypted_kind {
            Some(EncryptedFieldKind::String) => quote! {
                if let Some(ref value) = self.#field {
                    exec = exec.bind(rullst_orm::privacy::encrypt_model_field(
                        value,
                        #table_name,
                        #field_str,
                    )?);
                }
            },
            Some(EncryptedFieldKind::OptionalString) => quote! {
                if let Some(ref value) = self.#field {
                    let encrypted_value = match value.as_deref() {
                        Some(plaintext) => Some(rullst_orm::privacy::encrypt_model_field(
                            plaintext,
                            #table_name,
                            #field_str,
                        )?),
                        None => None,
                    };
                    exec = exec.bind(encrypted_value);
                }
            },
            None => quote! {
                if let Some(ref value) = self.#field {
                    exec = exec.bind(value.clone());
                }
            },
        });

        apply_to_model.push(quote! {
            if let Some(ref val) = self.#field {
                self.model.#field = val.clone();
            }
        });
    }

    let policy_check = if !parsed.policy.is_empty() {
        let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
        quote! {
            if !<#policy_type as rullst_orm::Policy<#name>>::can_update(self.model).await? {
                return Err(rullst_orm::Error::Validation(
                    "Policy prevents updating this record".to_string()
                ));
            }
        }
    } else {
        quote! {}
    };

    let tenant_guard = if let Some(tenant_field_type) = tenant_field_type {
        let col_ident = syn::Ident::new(&parsed.tenant_column, name.span());
        let col = &parsed.tenant_column;
        quote! {
            let tenant = rullst_orm::tenant::get_tenant_id().ok_or_else(|| {
                rullst_orm::Error::Validation(format!(
                    "tenant context is required to update `{}`",
                    #table_name
                ))
            })?;
            let expected_tenant: #tenant_field_type = tenant.try_into().map_err(|_| {
                rullst_orm::Error::Validation(format!(
                    "tenant context type does not match `{}.{}`",
                    #table_name,
                    stringify!(#col_ident)
                ))
            })?;
            if self.model.#col_ident != expected_tenant {
                return Err(rullst_orm::Error::Validation(
                    "record is outside the active tenant scope".to_string()
                ));
            }
            sql.push_str(concat!(" AND ", #col, " = ?"));
        }
    } else {
        quote! {}
    };
    let tenant_binding = if !parsed.tenant_column.is_empty() {
        let col_ident = syn::Ident::new(&parsed.tenant_column, name.span());
        quote! { exec = exec.bind(self.model.#col_ident.clone()); }
    } else {
        quote! {}
    };
    let tenant_rows_check = if !parsed.tenant_column.is_empty() {
        quote! {
            if result.rows_affected() != 1 {
                return Err(rullst_orm::Error::Validation(
                    "record is outside the active tenant scope".to_string()
                ));
            }
        }
    } else {
        quote! {}
    };

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

                #policy_check

                let driver = rullst_orm::Orm::driver()?;
                let mut sql = format!("UPDATE {} SET {} WHERE id = ?", #table_name, sets.join(", "));
                #tenant_guard
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
                #tenant_binding
                let result = rullst_orm::execute_query!(exec, execute, pool)?;
                #tenant_rows_check

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
