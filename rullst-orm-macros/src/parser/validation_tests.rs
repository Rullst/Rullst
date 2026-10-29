//! Attribute and field-type validation of the model parser.

use super::{EncryptedFieldKind, ParsedModel, encrypted_field_kind, parse};
use syn::{DeriveInput, parse_quote};

fn parse_error(input: DeriveInput) -> String {
    match parse(&input) {
        Ok(_) => panic!("model should be rejected"),
        Err(error) => error.to_string(),
    }
}

fn parsed(input: DeriveInput) -> ParsedModel {
    parse(&input).expect("model should parse")
}

#[test]
fn only_option_of_a_plain_string_is_an_optional_encrypted_column() {
    assert_eq!(
        encrypted_field_kind(&parse_quote!(Option<String>)),
        Some(EncryptedFieldKind::OptionalString)
    );
    assert_eq!(encrypted_field_kind(&parse_quote!(Option<i64>)), None);
    assert_eq!(
        encrypted_field_kind(&parse_quote!(Option<String<u8>>)),
        None
    );
}

#[test]
fn attribute_values_are_limited_to_256_bytes() {
    let longest = format!("h{}", "x".repeat(255));
    let model: DeriveInput = parse_quote! {
        #[orm(before_save = #longest)]
        struct Post { id: i32 }
    };
    assert_eq!(parsed(model).before_save, longest);

    let too_long = format!("h{}", "x".repeat(256));
    assert_eq!(
        parse_error(parse_quote! {
            #[orm(before_save = #too_long)]
            struct Post { id: i32 }
        }),
        "attribute value exceeds 256 bytes"
    );
}

#[test]
fn soft_delete_fragments_are_limited_to_128_bytes() {
    let longest = "1".repeat(128);
    let model: DeriveInput = parse_quote! {
        #[orm(soft_delete(column = "archived", value = "0", delval = #longest))]
        struct Post { id: i32, archived: i32 }
    };
    let soft_delete = parsed(model).soft_delete.expect("soft delete");
    assert_eq!(soft_delete.delval, longest);

    let too_long = "1".repeat(129);
    assert_eq!(
        parse_error(parse_quote! {
            #[orm(soft_delete(column = "archived", value = "0", delval = #too_long))]
            struct Post { id: i32, archived: i32 }
        }),
        "soft-delete SQL fragments are limited to 128 bytes and cannot contain separators or comments"
    );
}

#[test]
fn misspelled_options_suggest_the_right_one() {
    assert_eq!(
        parse_error(parse_quote! { #[orm(tabel = "posts")] struct Post { id: i32 } }),
        "unknown model attribute; did you mean `#[orm(table = \"...\")]`?"
    );
    for (typo, expected) in [
        ("hasmany", "has_many"),
        ("belongsto", "belongs_to"),
        ("hasone", "has_one"),
        ("foreignkey", "foreign_key"),
        ("fk", "foreign_key"),
    ] {
        let option = syn::Ident::new(typo, proc_macro2::Span::call_site());
        assert_eq!(
            parse_error(parse_quote! {
                struct Post { id: i32, #[orm(#option = "User")] user: Option<User> }
            }),
            format!(
                "unknown attribute `#[orm({typo} = ...)]`; did you mean `#[orm({expected} = \"...\")]`?"
            )
        );
    }
}

#[test]
fn rag_context_and_json_markers_are_field_options() {
    let model = parsed(parse_quote! {
        struct Post {
            id: i32,
            #[orm(rag_context)]
            body: String,
            #[orm(json)]
            meta: String,
        }
    });
    let rag: Vec<String> = model
        .rag_context_fields
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(rag, ["body"]);
}

#[test]
fn sqlx_json_accepts_one_nullable_modifier() {
    let error = "SQLx json accepts only one optional `nullable` modifier";
    assert_eq!(
        parse_error(parse_quote! {
            struct Post { id: i32, #[sqlx(json(nullable, nullable))] meta: Option<Meta> }
        }),
        error
    );
    assert_eq!(
        parse_error(parse_quote! {
            struct Post { id: i32, #[sqlx(json(strict))] meta: Meta }
        }),
        error
    );
    parsed(parse_quote! {
        struct Post { id: i32, #[sqlx(json(nullable))] meta: Option<Meta> }
    });
}

#[test]
fn embedding_targets_must_be_persisted() {
    assert_eq!(
        parse_error(parse_quote! {
            struct Doc {
                id: i32,
                body: String,
                #[sqlx(skip)]
                #[orm(embedding_for = "body")]
                embedding: Option<Vector>,
            }
        }),
        "embedding_for requires a persisted non-relation field"
    );
}

#[test]
fn qualified_type_paths_are_not_bindable_scalars() {
    assert_eq!(
        parse_error(parse_quote! {
            #[orm(tenant_column = "tenant_id")]
            struct Post { id: i32, tenant_id: <Tenant as Key>::String }
        }),
        "tenant_column supports String, i32, f64, or bool so it can be bound without lossy conversion"
    );
    assert_eq!(
        parse_error(parse_quote! {
            struct Comment {
                id: i32,
                commentable_id: <Owner as Key>::i32,
                commentable_type: String,
                #[orm(morph_to = "Commentable", morph_name = "commentable")]
                commentable: Option<Commentable>,
            }
        }),
        "morph_to id fields support String, i32, f64, or bool so they can be bound without lossy conversion"
    );
    let discriminator = parse_error(parse_quote! {
        struct Comment {
            id: i32,
            commentable_id: i32,
            commentable_type: <Owner as Kind>::String,
            #[orm(morph_to = "Commentable", morph_name = "commentable")]
            commentable: Option<Commentable>,
        }
    });
    assert!(
        discriminator.starts_with("morph_to discriminator"),
        "{discriminator}"
    );
}
