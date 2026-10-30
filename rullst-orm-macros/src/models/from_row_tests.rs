use syn::{DeriveInput, parse_quote};

fn generated(input: DeriveInput) -> String {
    let parsed = crate::parser::parse(&input).expect("parse model");
    super::generate(&parsed).to_string()
}

fn compact(tokens: &str) -> String {
    tokens.split_whitespace().collect()
}

#[test]
fn a_plain_model_reads_every_field_by_name_through_the_reexported_sqlx() {
    let code = compact(&generated(parse_quote! {
        #[orm(table = "users")]
        struct User { id: i32, name: String, #[orm(skip)] r#type: Option<String> }
    }));
    assert!(code.contains(
        "impl<'__rullst_row,__RullstRow>rullst_orm::_sqlx::FromRow<'__rullst_row,__RullstRow>forUser"
    ));
    assert!(code.contains("__RullstRow:rullst_orm::_sqlx::Row"));
    assert!(code.contains("rullst_orm::_sqlx::ColumnIndex<__RullstRow>"));
    assert!(
        code.contains("letid:i32=rullst_orm::_sqlx::Row::try_get::<i32,_>(__rullst_row,\"id\")?;")
    );
    assert!(code.contains("try_get::<String,_>(__rullst_row,\"name\")?;"));
    // A raw identifier (allowed on an `#[orm(skip)]` field, which is still
    // read) names its column without the `r#` prefix.
    assert!(code.contains("try_get::<Option<String>,_>(__rullst_row,\"type\")?;"));
    assert!(code.contains("i32:rullst_orm::_sqlx::Decode<'__rullst_row,"));
    assert!(code.contains("Ok(Self{id,name,r#type})"));
    // The generated code names nothing the application must depend on.
    assert!(!code.contains("::sqlx::"));
    assert!(!code.contains("Default::default()"));
}

#[test]
fn skipped_fields_and_relations_start_from_default_without_reading_the_row() {
    let code = compact(&generated(parse_quote! {
        struct Post {
            id: i32,
            #[sqlx(skip)]
            snapshot: RowSnapshot,
            #[sqlx(default, skip)]
            cache: String,
            #[orm(belongs_to = "User", foreign_key = "user_id")]
            author: Option<User>,
            user_id: i32,
            // `#[orm(skip)]` keeps writes away from the column; the row is
            // still read, as with SQLx's derive.
            #[orm(skip)]
            computed: i64,
        }
    }));
    for field in [
        "snapshot:RowSnapshot",
        "cache:String",
        "author:Option<User>",
    ] {
        assert!(
            code.contains(&format!("let{field}=::core::default::Default::default();")),
            "{field}: {code}"
        );
    }
    for column in ["\"snapshot\"", "\"cache\"", "\"author\""] {
        assert!(!code.contains(column), "{column} must not be read: {code}");
    }
    assert!(code.contains("try_get::<i64,_>(__rullst_row,\"computed\")?;"));
    assert!(!code.contains("RowSnapshot:rullst_orm::_sqlx::Decode"));
}

#[test]
fn json_columns_decode_through_sqlx_json_and_nullable_ones_flatten_null() {
    let code = compact(&generated(parse_quote! {
        struct Post {
            id: i32,
            #[sqlx(json)]
            meta: Meta,
            #[sqlx(json(nullable))]
            extra: Option<Meta>,
        }
    }));
    assert!(code.contains(
        "try_get::<rullst_orm::_sqlx::types::Json<Meta>,_>(__rullst_row,\"meta\").map(|value|value.0)?;"
    ));
    assert!(code.contains("rullst_orm::_sqlx::types::Json<Meta>:rullst_orm::_sqlx::Decode"));
    assert!(code.contains(
        "try_get::<::core::option::Option<rullst_orm::_sqlx::types::Json<Option<Meta>>>,_>(__rullst_row,\"extra\").map(|value|value.and_then(|json|json.0))?;"
    ));
}

#[test]
fn a_missing_column_falls_back_only_where_a_default_is_declared() {
    let field_default = compact(&generated(parse_quote! {
        struct Post { id: i32, #[sqlx(default)] views: i64 }
    }));
    assert!(field_default.contains(
        "letviews:i64=rullst_orm::_sqlx::Row::try_get::<i64,_>(__rullst_row,\"views\").or_else(|error|matcherror{rullst_orm::_sqlx::Error::ColumnNotFound(_)=>::core::result::Result::Ok(::core::default::Default::default()),error=>::core::result::Result::Err(error),})?;"
    ));
    assert!(
        field_default
            .contains("letid:i32=rullst_orm::_sqlx::Row::try_get::<i32,_>(__rullst_row,\"id\")?;")
    );
    assert!(!field_default.contains("__rullst_default"));

    let container_default = compact(&generated(parse_quote! {
        #[sqlx(default)]
        struct Post { id: i32, #[sqlx(default)] views: i64, title: String }
    }));
    assert!(
        container_default
            .contains("let__rullst_default=<Selfas::core::default::Default>::default();")
    );
    assert!(container_default.contains("Post:::core::default::Default"));
    assert!(
        container_default
            .contains("ColumnNotFound(_)=>::core::result::Result::Ok(__rullst_default.title)")
    );
    assert!(
        container_default
            .contains("ColumnNotFound(_)=>::core::result::Result::Ok(__rullst_default.id)")
    );
    // A field-level default takes `Default::default()`, as in SQLx.
    assert!(container_default.contains(
        "\"views\").or_else(|error|matcherror{rullst_orm::_sqlx::Error::ColumnNotFound(_)=>::core::result::Result::Ok(::core::default::Default::default())"
    ));
}

#[test]
fn custom_from_row_leaves_the_impl_to_the_model() {
    let parsed = crate::parser::parse(&parse_quote! {
        #[orm(table = "users", custom_from_row)]
        struct User { id: i32 }
    })
    .expect("custom_from_row parses");
    assert!(parsed.custom_from_row);
    assert!(super::generate(&parsed).is_empty());

    let duplicate: DeriveInput = parse_quote! {
        #[orm(custom_from_row, custom_from_row)]
        struct User { id: i32 }
    };
    let error = match crate::parser::parse(&duplicate) {
        Ok(_) => panic!("a repeated custom_from_row must fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("duplicate"), "{error}");

    let valued: DeriveInput = parse_quote! {
        #[orm(custom_from_row = "yes")]
        struct User { id: i32 }
    };
    assert!(crate::parser::parse(&valued).is_err());
}
