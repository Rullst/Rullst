use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::{DeriveInput, parse_quote};

/// The whole `#[derive(Orm)]` expansion of `input`, as `rullst_macro` builds it.
fn expansion(input: &DeriveInput) -> TokenStream {
    let parsed = crate::parser::parse(input).expect("parse model");
    let relationships = crate::relationships::generate(&parsed);
    let builder = crate::builder::generate(
        &parsed,
        &relationships.flags,
        &relationships.inits,
        &relationships.methods,
        &relationships.eager_loads,
    );
    let factory_observer = crate::factory_observer::generate(&parsed);
    let model = crate::models::generate(&parsed, &relationships.model_methods);
    quote! { #builder #factory_observer #model }
}

fn visit(stream: TokenStream, visitor: &mut impl FnMut(&[TokenTree])) {
    let tokens = stream.into_iter().collect::<Vec<_>>();
    visitor(&tokens);
    for token in tokens {
        if let TokenTree::Group(group) = token {
            visit(group.stream(), visitor);
        }
    }
}

/// Every identifier of the expansion, at any depth.
fn identifiers(stream: TokenStream) -> Vec<String> {
    let mut found = Vec::new();
    visit(stream, &mut |tokens| {
        for token in tokens {
            if let TokenTree::Ident(ident) = token {
                found.push(ident.to_string());
            }
        }
    });
    found
}

/// The arguments of every `info_span!(…)` of the expansion.
fn span_arguments(stream: TokenStream) -> Vec<String> {
    let mut found = Vec::new();
    visit(stream, &mut |tokens| {
        for window in tokens.windows(3) {
            if let [
                TokenTree::Ident(name),
                TokenTree::Punct(bang),
                TokenTree::Group(arguments),
            ] = window
                && name == "info_span"
                && bang.as_char() == '!'
            {
                found.push(arguments.stream().to_string());
            }
        }
    });
    found
}

fn models() -> Vec<DeriveInput> {
    vec![
        parse_quote! {
            #[derive(Orm)]
            pub struct Lesson { pub id: i32, pub title: String }
        },
        parse_quote! {
            #[derive(Orm)]
            #[orm(table = "lessons", tenant_column = "tenant_id", auditable, policy = "LessonPolicy")]
            pub struct Lesson {
                pub id: i32,
                pub tenant_id: String,
                pub title: String,
                pub deleted_at: Option<String>,
                #[sqlx(skip)]
                pub snapshot: RowSnapshot,
            }
        },
    ]
}

#[test]
fn the_span_is_built_before_the_body_and_entered_only_while_enabled() {
    let generated = super::instrument_query(
        &format_ident!("Lesson"),
        "lessons",
        "paginate",
        &["page", "per_page"],
        quote! { Ok(()) },
    )
    .to_string();
    assert_eq!(
        generated,
        quote! {
            let __rullst_query_span = rullst_orm::_tracing::info_span!(
                target: "rullst_orm",
                "rullst.orm.query",
                page = page,
                per_page = per_page,
                orm.model = stringify!(Lesson),
                orm.table = "lessons",
                orm.operation = "paginate"
            );
            let __rullst_query = async move { Ok(()) };
            if __rullst_query_span.is_disabled() {
                __rullst_query.await
            } else {
                rullst_orm::_tracing::Instrument::instrument(__rullst_query, __rullst_query_span).await
            }
        }
        .to_string()
    );
}

#[test]
fn derived_models_name_tracing_only_through_rullst_orm() {
    for input in models() {
        let generated = expansion(&input);
        let text = generated.to_string();
        // `#[tracing::instrument]` expands to `::tracing` paths, which only a
        // direct `tracing` dependency of the application resolves.
        assert!(!text.contains("_tracing :: instrument"), "{text}");
        assert!(
            !identifiers(generated)
                .iter()
                .any(|ident| ident == "tracing"),
            "{text}"
        );
        assert!(text.contains("rullst_orm :: _tracing :: info_span !"));
    }
}

#[test]
fn every_query_span_keeps_the_fields_of_the_former_instrument_attribute() {
    let operations = [
        "select_many",
        "select_many_with_tx",
        "select_first",
        "select_first_with_tx",
        "paginate",
        "count",
        "chunk",
        "chunk_by_id",
        "delete_all",
        "delete_all_with_tx",
        "delete",
        "delete_with_tx",
        "save",
        "save_with_tx",
        "force_delete",
        "restore",
    ];
    for input in models() {
        let spans = span_arguments(expansion(&input));
        assert_eq!(spans.len(), operations.len(), "{spans:#?}");
        for operation in operations {
            // `instrument` recorded the parameters it did not skip, by value.
            let recorded = match operation {
                "paginate" => "page = page , per_page = per_page , ",
                "chunk" | "chunk_by_id" => "size = size , ",
                _ => "",
            };
            let expected = format!(
                "target : \"rullst_orm\" , \"rullst.orm.query\" , {recorded}orm . model = \
                 stringify ! (Lesson) , orm . table = \"lessons\" , orm . operation = \
                 \"{operation}\""
            );
            assert_eq!(
                spans.iter().filter(|span| **span == expected).count(),
                1,
                "{operation}: {spans:#?}"
            );
        }
    }
}
