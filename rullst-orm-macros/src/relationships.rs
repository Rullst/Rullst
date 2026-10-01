use crate::parser::{ParsedModel, ParsedRelation};
use proc_macro2::TokenStream;
use quote::quote;

mod eager;
mod eager_assign;
mod lazy;

/// The foreign-key field of a relation. An omitted `foreign_key` names the
/// model on the other side of the key: a `belongs_to` key lives on this model
/// and defaults to `<related model>_id` (`post_id` for `belongs_to = "Post"`),
/// while a has-one/has-many key lives on the related table and defaults to
/// `<this model>_id`.
fn foreign_key_field(name: &syn::Ident, rel: &ParsedRelation) -> String {
    if !rel.foreign_key.is_empty() {
        rel.foreign_key.clone()
    } else if rel.rel_type == "belongs_to" {
        format!("{}_id", rel.rel_model.to_lowercase())
    } else {
        format!("{}_id", name.to_string().to_lowercase())
    }
}

/// Identifiers shared by the lazy and eager loaders of one relation.
struct RelationNames<'a> {
    rel: &'a ParsedRelation,
    method_name: syn::Ident,
    rel_model_ident: syn::Ident,
    rel_model_builder_ident: syn::Ident,
    /// The foreign-key field (see [`foreign_key_field`]).
    fk_ident: syn::Ident,
    /// This model's matched key (`local_key`, default `id`).
    lk_ident: syn::Ident,
    /// The related model's matched key (`related_key`, default `id`).
    pk_ident: syn::Ident,
    morph_id_ident: syn::Ident,
    morph_type_ident: syn::Ident,
}

impl<'a> RelationNames<'a> {
    fn new(name: &syn::Ident, rel: &'a ParsedRelation) -> Self {
        let or_id = |key: &str| {
            if key.is_empty() {
                "id".to_string()
            } else {
                key.to_string()
            }
        };
        let morph_id = if rel.foreign_key.is_empty() {
            format!("{}_id", rel.morph_name)
        } else {
            rel.foreign_key.clone()
        };
        Self {
            rel,
            method_name: quote::format_ident!("{}", rel.field_name),
            rel_model_ident: syn::Ident::new(&rel.rel_model, rel.field_name.span()),
            rel_model_builder_ident: quote::format_ident!("{}QueryBuilder", rel.rel_model),
            fk_ident: quote::format_ident!("{}", foreign_key_field(name, rel)),
            lk_ident: quote::format_ident!("{}", or_id(&rel.local_key)),
            pk_ident: quote::format_ident!("{}", or_id(&rel.related_key)),
            morph_id_ident: quote::format_ident!("{}", morph_id),
            morph_type_ident: quote::format_ident!("{}_type", rel.morph_name),
        }
    }
}

pub struct GeneratedRelationships {
    pub flags: Vec<TokenStream>,
    pub inits: Vec<TokenStream>,
    pub methods: Vec<TokenStream>,
    pub model_methods: Vec<TokenStream>,
    pub eager_loads: TokenStream,
}

#[cfg_attr(test, mutants::skip)]
pub fn generate(parsed: &ParsedModel) -> GeneratedRelationships {
    let mut flags = vec![];
    let mut inits = vec![];
    let mut methods = vec![];
    let mut model_methods = vec![];
    let mut eager_loads = vec![];

    let name = &parsed.name;

    for rel in &parsed.relations {
        let names = RelationNames::new(name, rel);
        let field_name = &rel.field_name;
        let load_flag_ident = quote::format_ident!("load_{}", field_name);
        let filter_flag_ident = quote::format_ident!("filter_{}", field_name);
        let rel_model_builder_ident = &names.rel_model_builder_ident;

        flags.push(quote! {
            pub #load_flag_ident: bool,
            pub #filter_flag_ident: Option<std::sync::Arc<dyn Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync>>,
        });
        inits.push(quote! {
            #load_flag_ident: false,
            #filter_flag_ident: None,
        });

        let with_method_ident = quote::format_ident!("with_{}", field_name);
        let with_constrained_method_ident = quote::format_ident!("with_{}_constrained", field_name);
        methods.push(quote! {
            pub fn #with_method_ident(mut self) -> Self {
                self.#load_flag_ident = true;
                self
            }
            pub fn #with_constrained_method_ident<F>(mut self, filter: F) -> Self
            where F: Fn(#rel_model_builder_ident) -> #rel_model_builder_ident + Send + Sync + 'static {
                self.#load_flag_ident = true;
                self.#filter_flag_ident = Some(std::sync::Arc::new(filter));
                self
            }
        });

        model_methods.push(lazy::generate(name, &names));
        eager_loads.push(eager::generate(name, &names));
    }

    GeneratedRelationships {
        flags,
        inits,
        methods,
        model_methods,
        eager_loads: quote! { #(#eager_loads)* },
    }
}
