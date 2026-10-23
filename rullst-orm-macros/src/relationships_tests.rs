use syn::{DeriveInput, parse_quote};

fn loaders(input: DeriveInput) -> (String, String) {
    let parsed = crate::parser::parse(&input).expect("parse model");
    let generated = super::generate(&parsed);
    let lazy = generated
        .model_methods
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    (lazy, generated.eager_loads.to_string())
}

#[test]
fn omitted_foreign_keys_name_the_model_on_the_other_side() {
    let (lazy, eager) = loaders(parse_quote! {
        struct Comment {
            id: i32,
            post_id: i32,
            #[orm(belongs_to = "Post")]
            post: Option<Post>,
            #[orm(has_many = "Reply")]
            replies: Option<Vec<Reply>>,
        }
    });
    // belongs_to reads this model's `post_id`; has_many filters the related
    // table by `comment_id`.
    assert!(lazy.contains("(& self . post_id)"), "{lazy}");
    assert!(lazy.contains("Reply :: query () . where_eq (stringify ! (comment_id) , key)"));
    assert!(
        eager.contains("where_in (stringify ! (comment_id) , parent_ids)"),
        "{eager}"
    );
    assert!(!lazy.contains("xyzzy") && !eager.contains("xyzzy"));
}

#[test]
fn relation_keys_go_through_the_nullable_key_helper() {
    let (lazy, eager) = loaders(parse_quote! {
        struct Comment {
            id: i32,
            post_id: Option<i32>,
            #[orm(belongs_to = "Post")]
            post: Option<Post>,
        }
    });
    let key =
        "(& rullst_orm :: __relation_key :: Key (& self . post_id)) . __rullst_relation_key ()";
    assert!(lazy.contains(key), "{lazy}");
    assert!(
        eager.contains("rullst_orm :: __relation_key :: Key (& m . post_id)"),
        "{eager}"
    );
    // Several parents may share one related row, so the batch is shared.
    assert!(eager.contains("struct __RullstEagerShare"), "{eager}");
}
