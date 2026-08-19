use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_ai_methods(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;

    let rag_impl = if !parsed.rag_context_fields.is_empty() {
        let rag_fields = &parsed.rag_context_fields;
        quote! {
            impl rullst_orm::RagContext for #name {
                fn get_context(&self) -> String {
                    let mut parts = Vec::new();
                    #(
                        parts.push(format!("{}: {}", stringify!(#rag_fields), self.#rag_fields));
                    )*
                    parts.join("\n")
                }
            }
        }
    } else {
        quote! {}
    };

    let save_with_embedding = if let Some((embedding_field, text_field)) = &parsed.embedding_for {
        let text_field_ident = syn::Ident::new(text_field, proc_macro2::Span::call_site());
        quote! {
            impl #name {
                #[cfg(feature = "ai")]
                pub async fn save_with_embedding(&mut self, client: &rullst_ai::AiClient) -> Result<(), rullst_orm::Error> {
                    let vector = client.embed(&self.#text_field_ident.to_string()).await.map_err(|e| rullst_orm::Error::DatabaseError(e.to_string()))?;
                    self.#embedding_field = Some(rullst_orm::_pgvector::Vector::from(vector));
                    self.save().await
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #rag_impl
        #save_with_embedding
    }
}
