use super::*;
use syn::parse_quote;

#[test]
fn generates_typed_metadata_and_explicit_widgets() {
    let input: DeriveInput = parse_quote! {
        #[nexus(table = "articles", label = "Articles", icon = "📰")]
        struct Article {
            #[nexus(primary_key)]
            uuid: String,
            #[nexus(kind = "textarea", label = "Article body")]
            body: String,
            #[nexus(kind = "enum", options = "draft, published")]
            status: String,
            published: bool,
        }
    };
    let output = expand_nexus(&input)
        .expect("valid Nexus derive")
        .to_string();
    assert!(output.contains("fn nexus_table"));
    assert!(output.contains("\"articles\""));
    assert!(output.contains("FieldKind :: Textarea"));
    assert!(output.contains("FieldKind :: Enum"));
    assert!(output.contains("\"draft\""));
    assert!(output.contains("FieldKind :: Boolean"));
    assert!(output.contains("\"uuid\""));
}

#[test]
fn rejects_invalid_widget_configuration_and_primary_key() {
    let invalid_kind: DeriveInput = parse_quote! {
        struct Article {
            id: i64,
            #[nexus(kind = "magic")]
            body: String,
        }
    };
    assert!(
        expand_nexus(&invalid_kind)
            .expect_err("invalid widget must fail")
            .to_string()
            .contains("unsupported Nexus field kind")
    );

    let invalid_pk: DeriveInput = parse_quote! {
        #[nexus(primary_key = "missing")]
        struct Article { id: i64 }
    };
    assert!(
        expand_nexus(&invalid_pk)
            .expect_err("missing primary key must fail")
            .to_string()
            .contains("is not a field")
    );

    let invalid_tenant: DeriveInput = parse_quote! {
        #[nexus(tenant = "organization_id")]
        struct Article { id: i64 }
    };
    assert!(
        expand_nexus(&invalid_tenant)
            .expect_err("missing tenant column must fail")
            .to_string()
            .contains("tenant column `organization_id` is not a field")
    );

    let invalid_tenant_type: DeriveInput = parse_quote! {
        #[nexus(tenant = "organization_id")]
        struct Article {
            id: i64,
            organization_id: Option<String>,
        }
    };
    assert!(
        expand_nexus(&invalid_tenant_type)
            .expect_err("nullable tenant column must fail")
            .to_string()
            .contains("must use a non-optional `String` with text metadata")
    );

    let invalid_tenant_widget: DeriveInput = parse_quote! {
        #[nexus(tenant = "organization_id")]
        struct Article {
            id: i64,
            #[nexus(kind = "textarea")]
            organization_id: String,
        }
    };
    assert!(
        expand_nexus(&invalid_tenant_widget)
            .expect_err("non-text tenant metadata must fail")
            .to_string()
            .contains("must use a non-optional `String` with text metadata")
    );
}

#[test]
fn skips_shared_orm_options_and_relation_fields() {
    let input: DeriveInput = parse_quote! {
        #[orm(
            table_name = "projects",
            tenant_column = "tenant_id",
            policy = "ProjectPolicy",
            soft_delete(column = "removed_at"),
            auditable
        )]
        struct Project {
            id: i32,
            tenant_id: String,
            #[orm(has_many = "Tag", foreign_key = "project_id", cascade_soft_delete)]
            tags: Option<Vec<Tag>>,
            #[orm(belongs_to = "Owner", foreign_key = "owner_id")]
            owner: Option<Owner>,
            owner_id: i32,
            #[orm(masked)]
            title: String,
            removed_at: Option<String>,
        }
    };
    let output = expand_nexus(&input)
        .expect("shared ORM options must not break the Nexus derive")
        .to_string();
    assert!(output.contains("fn nexus_table () -> & 'static str { \"projects\" }"));
    for column in ["id", "tenant_id", "owner_id", "title", "removed_at"] {
        assert!(output.contains(&format!("name : \"{column}\"")), "{column}");
    }
    for relation in ["tags", "owner"] {
        assert!(
            !output.contains(&format!("name : \"{relation}\"")),
            "{relation}"
        );
    }
    assert!(!output.contains("fn nexus_tenant_column"));
}

#[test]
fn generates_readonly_hidden_tenant_scope_from_orm_metadata() {
    let input: DeriveInput = parse_quote! {
        #[orm(table = "articles", tenant = "organization_id")]
        struct Article {
            id: i64,
            organization_id: String,
            title: String,
        }
    };
    let output = expand_nexus(&input)
        .expect("valid tenant-scoped Nexus derive")
        .to_string();

    assert!(output.contains("fn nexus_tenant_column"));
    assert!(output.contains("Some (\"organization_id\")"));
    let tenant_field = output
        .split("name : \"organization_id\"")
        .nth(1)
        .expect("tenant field metadata emitted");
    assert!(tenant_field.contains("hidden : true"));
    assert!(tenant_field.contains("readonly : true"));
}

/// The generated `FieldMeta` of `name`, or `None` when it is not emitted.
fn field_meta(output: &str, name: &str) -> Option<String> {
    let rest = output.split(&format!("name : \"{name}\"")).nth(1)?;
    Some(rest.split('}').next().unwrap_or_default().to_string())
}

#[test]
fn honours_orm_skip_and_confidentiality_markers() {
    let input: DeriveInput = parse_quote! {
        #[orm(table = "users")]
        struct User {
            id: i32,
            name: String,
            #[orm(skip)]
            display_name: String,
            #[sqlx(default, skip)]
            cached_total: i64,
            #[orm(hidden)]
            reset_token: String,
            #[orm(encrypted)]
            tax_id: String,
            cpf: SecretString,
            backup_cpf: Option<rullst_orm::SecretString>,
            #[orm(masked)]
            api_token: String,
            #[orm(masked)]
            #[nexus(kind = "text")]
            recovery_hint: String,
            #[orm(hidden)]
            #[nexus(kind = "password")]
            password: String,
            #[sqlx(json)]
            settings: Json<Settings>,
        }
    };
    let output = expand_nexus(&input)
        .expect("ORM markers are valid Nexus input")
        .to_string();

    for skipped in ["display_name", "cached_total"] {
        assert!(field_meta(&output, skipped).is_none(), "{skipped}");
    }
    for sealed in ["reset_token", "tax_id", "cpf", "backup_cpf"] {
        let meta = field_meta(&output, sealed).expect("protected field metadata");
        assert!(meta.contains("FieldKind :: Password"), "{sealed}: {meta}");
        assert!(meta.contains("hidden : true"), "{sealed}: {meta}");
        assert!(meta.contains("readonly : true"), "{sealed}: {meta}");
    }
    for write_only in ["api_token", "password"] {
        let meta = field_meta(&output, write_only).expect("password field metadata");
        assert!(
            meta.contains("FieldKind :: Password"),
            "{write_only}: {meta}"
        );
        assert!(meta.contains("hidden : false"), "{write_only}: {meta}");
        assert!(meta.contains("readonly : false"), "{write_only}: {meta}");
    }
    let relaxed = field_meta(&output, "recovery_hint").expect("masked override");
    assert!(relaxed.contains("FieldKind :: Text"), "{relaxed}");
    let settings = field_meta(&output, "settings").expect("json column is kept");
    assert!(settings.contains("hidden : false"), "{settings}");
}

#[test]
fn rejects_widgets_that_would_expose_protected_orm_fields() {
    let cases: [DeriveInput; 3] = [
        parse_quote! {
            struct Patient {
                id: i32,
                #[orm(encrypted)]
                #[nexus(kind = "textarea")]
                diagnosis: String,
            }
        },
        parse_quote! {
            struct Patient {
                id: i32,
                #[nexus(kind = "enum", options = "a, b")]
                cpf: SecretString,
            }
        },
        parse_quote! {
            struct Account {
                id: i32,
                #[orm(hidden)]
                #[nexus(kind = "text")]
                reset_token: String,
            }
        },
    ];
    for input in cases {
        let error = expand_nexus(&input)
            .expect_err("a widget override must not expose a protected field")
            .to_string();
        assert!(error.contains("hidden"), "{error}");
    }
}
