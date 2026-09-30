//! Optional generated APIs follow the ORM runtime's features.
//!
//! A `#[cfg(feature = "...")]` written into macro output is evaluated in the
//! crate that invokes the derive, not in `rullst-orm`, so an application that
//! enabled `rullst-orm/redis` never received the Redis methods unless it also
//! declared a `redis` feature of its own. The matching runtime opts into
//! `runtime-feature-gates` and forwards its `redis` and `ai` features here, so
//! the decision is made at expansion time. Without that opt-in (an older 12.x
//! runtime resolving a newer macro crate) the legacy attributes are emitted
//! unchanged.

use proc_macro2::TokenStream;
use quote::quote;

/// Attribute placed before Redis-only generated items and statements.
pub(crate) fn redis() -> TokenStream {
    attribute(
        "redis",
        cfg!(feature = "runtime-feature-gates"),
        cfg!(feature = "redis"),
    )
}

/// Attribute placed before the `rullst-ai` embedding helper.
pub(crate) fn ai() -> TokenStream {
    attribute(
        "ai",
        cfg!(feature = "runtime-feature-gates"),
        cfg!(feature = "ai"),
    )
}

/// `runtime_gates` selects expansion-time gating; `enabled` is the runtime's
/// forwarded feature. A disabled item keeps its tokens under `cfg(any())`, the
/// always-false configuration, exactly like the legacy attribute when the
/// invoking crate lacks the feature.
fn attribute(feature: &str, runtime_gates: bool, enabled: bool) -> TokenStream {
    match (runtime_gates, enabled) {
        (false, _) => quote! { #[cfg(feature = #feature)] },
        (true, true) => TokenStream::new(),
        (true, false) => quote! { #[cfg(any())] },
    }
}

#[cfg(test)]
mod tests {
    use super::attribute;

    #[test]
    fn runtime_gates_never_emit_an_application_feature_check() {
        assert_eq!(
            attribute("redis", false, true).to_string(),
            "# [cfg (feature = \"redis\")]"
        );
        assert!(attribute("redis", true, true).is_empty());
        assert_eq!(
            attribute("redis", true, false).to_string(),
            "# [cfg (any ())]"
        );
        assert_eq!(attribute("ai", true, false).to_string(), "# [cfg (any ())]");
    }

    #[test]
    fn generated_models_follow_the_runtime_features() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[orm(table = "docs")]
            struct Doc {
                id: i32,
                body: String,
                #[orm(embedding_for = "body")]
                embedding: Option<Vector>,
            }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let rels = crate::relationships::generate(&parsed);
        let generated = format!(
            "{} {}",
            crate::builder::generate(
                &parsed,
                &rels.flags,
                &rels.inits,
                &rels.methods,
                &rels.eager_loads
            ),
            crate::models::generate(&parsed, &rels.model_methods)
        );
        assert!(generated.contains("fn remember"));
        assert!(generated.contains("fn save_to_redis"));
        assert!(generated.contains("fn save_with_embedding"));
        let application_cfg = generated.contains("cfg (feature =");
        assert_eq!(
            application_cfg,
            !cfg!(feature = "runtime-feature-gates"),
            "{generated}"
        );
    }
}
