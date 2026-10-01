use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

#[cfg_attr(test, mutants::skip)]
pub fn derive_personal_data_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// The table `#[derive(Orm)]` uses: `#[orm(table = "...")]` (or its
/// `table_name` alias), else the `<struct>s` default.
fn table_name(input: &DeriveInput) -> syn::Result<String> {
    let mut table = None;
    for attribute in input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("orm"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("table") || meta.path.is_ident("table_name") {
                table = Some(meta.value()?.parse::<LitStr>()?.value());
                Ok(())
            } else {
                crate::nexus::skip_orm_option(&meta)
            }
        })?;
    }
    Ok(table.unwrap_or_else(|| format!("{}s", input.ident.to_string().to_lowercase())))
}

/// Whether the column is stored encrypted: `#[orm(encrypted)]` or a
/// `SecretString` type, unless `#[orm(skip)]`/`#[sqlx(skip)]` leaves it out
/// of the table. `#[privacy]` alone only redacts `Debug`.
fn is_encrypted_column(field: &syn::Field) -> syn::Result<bool> {
    let mut encrypted = crate::parser::is_secret_string_type(&field.ty);
    let mut skipped = false;
    for attribute in &field.attrs {
        let orm = attribute.path().is_ident("orm");
        if !orm && !attribute.path().is_ident("sqlx") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            encrypted |= orm && meta.path.is_ident("encrypted");
            skipped |= meta.path.is_ident("skip");
            crate::nexus::skip_orm_option(&meta)
        })?;
    }
    Ok(encrypted && !skipped)
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let table_name = table_name(input)?;

    // Collect the struct fields to inspect privacy tags
    let mut debug_fields = quote! {};
    let mut encrypted_fields = Vec::new();
    let mut personal_fields = Vec::new();

    if let Data::Struct(data_struct) = &input.data
        && let Fields::Named(fields_named) = &data_struct.fields
    {
        for field in &fields_named.named {
            let Some(field_name) = field.ident.clone() else {
                return Err(syn::Error::new_spanned(
                    field,
                    "PersonalData requires every named field to have an identifier",
                ));
            };

            if is_encrypted_column(field)? {
                encrypted_fields.push(field_name.to_string());
            }

            // Check if the field has the `#[privacy]` attribute
            let has_privacy = field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("privacy"));

            if has_privacy {
                personal_fields.push(field_name.to_string());
                // If it's sensitive, mask the output in standard log/debug
                debug_fields = quote! {
                    #debug_fields
                    .field(stringify!(#field_name), &"[REDACTED_BY_RULLST_SHIELD]")
                };
            } else {
                // If it's public, display it normally
                debug_fields = quote! {
                    #debug_fields
                    .field(stringify!(#field_name), &self.#field_name)
                };
            }
        }
    }
    let has_encrypted_data = !encrypted_fields.is_empty();

    // Rust code that will be injected into the user's application
    Ok(quote! {
        // Override standard Debug to prevent leakage in logs (Log Leaking Protection)
        impl std::fmt::Debug for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #debug_fields
                    .finish()
            }
        }

        // Register the model in the Rullst compliance engine
        impl rullst_orm::privacy::ComplianceModel for #name {
            fn compliance_schema() -> rullst_orm::privacy::PrivacyReport {
                rullst_orm::privacy::PrivacyReport {
                    table_name: #table_name.to_string(),
                    has_encrypted_data: #has_encrypted_data,
                    encrypted_fields: vec![ #( #encrypted_fields ),* ],
                }
            }

            fn personal_fields() -> Vec<&'static str> {
                vec![ #( #personal_fields ),* ]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn reports_only_encrypted_columns_and_the_orm_table() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "people", auditable)]
            struct Person {
                id: i32,
                #[privacy]
                ssn: String,
                #[privacy]
                #[orm(encrypted)]
                tax_id: Option<String>,
                api_key: SecretString,
                #[sqlx(skip)]
                #[orm(skip)]
                scratch: SecretString,
            }
        };
        let expanded = super::expand(&input).expect("expand").to_string();
        assert!(expanded.contains("table_name : \"people\" . to_string ()"));
        assert!(expanded.contains("has_encrypted_data : true"));
        assert!(expanded.contains("encrypted_fields : vec ! [\"tax_id\" , \"api_key\"]"));
        assert!(expanded.contains("vec ! [\"ssn\" , \"tax_id\"]"));

        let plain: DeriveInput = parse_quote! {
            struct UserData { id: i32, #[privacy] ssn: String }
        };
        let expanded = super::expand(&plain).expect("expand").to_string();
        assert!(expanded.contains("table_name : \"userdatas\" . to_string ()"));
        assert!(expanded.contains("has_encrypted_data : false"));
    }
}
