use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[cfg_attr(test, mutants::skip)]
pub fn derive_personal_data_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Collect the struct fields to inspect privacy tags
    let mut debug_fields = quote! {};
    let mut encrypted_fields = Vec::new();
    let mut has_encrypted_data = false;

    if let Data::Struct(data_struct) = input.data
        && let Fields::Named(fields_named) = data_struct.fields
    {
        for field in fields_named.named {
            let field_name = field.ident.unwrap();

            // Check if the field has the `#[privacy]` attribute
            let has_privacy = field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("privacy"));

            if has_privacy {
                has_encrypted_data = true;
                encrypted_fields.push(field_name.to_string());
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

    // Convert string fields to a slice expression
    let encrypted_fields_tokens = encrypted_fields.iter().map(|f| quote! { #f });

    // Rust code that will be injected into the user's application
    let expanded = quote! {
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
                    table_name: stringify!(#name).to_lowercase(),
                    has_encrypted_data: #has_encrypted_data,
                    encrypted_fields: vec![ #( #encrypted_fields_tokens ),* ],
                }
            }
        }
    };

    TokenStream::from(expanded)
}
