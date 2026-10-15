use super::super::expand_nexus;
use syn::{DeriveInput, parse_quote};

/// The generated `FieldMeta` literal of `name`, up to its `readonly` flag.
fn field_meta(output: &str, name: &str) -> Option<String> {
    let rest = output.split(&format!("name : \"{name}\"")).nth(1)?;
    let end = rest.find("readonly : ")? + "readonly : ".len() + 5;
    rest.get(..end).map(str::to_string)
}

#[test]
fn several_key_columns_declare_a_composite_key() {
    let annotated: DeriveInput = parse_quote! {
        struct Membership {
            #[nexus(primary_key)]
            user_id: i32,
            role: String,
            #[nexus(primary_key)]
            team_id: i64,
        }
    };
    let output = expand_nexus(&annotated)
        .expect("valid Nexus derive")
        .to_string();
    assert!(output.contains("fn nexus_pk () -> & 'static str { \"user_id\" }"));
    assert!(output.contains(
        "fn nexus_pk_columns () -> Vec < & 'static str > { vec ! [\"user_id\" , \"team_id\"] }"
    ));
    // Composite key columns are entered on create, even integer ones.
    for column in ["user_id", "team_id"] {
        let meta = field_meta(&output, column).expect("key metadata");
        assert!(
            meta.contains("hidden : false") && meta.contains("readonly : false"),
            "{meta}"
        );
    }

    let listed: DeriveInput = parse_quote! {
        #[nexus(primary_key = " lesson_id ,user_id")]
        struct LessonProgress { user_id: i32, lesson_id: i32, percent: i32 }
    };
    let output = expand_nexus(&listed)
        .expect("valid Nexus derive")
        .to_string();
    assert!(output.contains("fn nexus_pk () -> & 'static str { \"lesson_id\" }"));
    assert!(
        output.contains("vec ! [\"lesson_id\" , \"user_id\"]"),
        "{output}"
    );

    // Annotations agreeing with the list, in order, are accepted.
    let both: DeriveInput = parse_quote! {
        #[nexus(primary_key = "user_id, lesson_id")]
        struct LessonProgress {
            #[nexus(primary_key)]
            user_id: i32,
            #[nexus(primary_key)]
            lesson_id: i32,
        }
    };
    assert!(expand_nexus(&both).is_ok());
}

fn derive_error(input: DeriveInput) -> String {
    expand_nexus(&input)
        .expect_err("invalid composite key must fail")
        .to_string()
}

#[test]
fn composite_key_lists_are_distinct_bounded_and_consistent() {
    assert_eq!(
        derive_error(parse_quote! {
            #[nexus(primary_key = "user_id,,lesson_id")]
            struct LessonProgress { user_id: i32, lesson_id: i32 }
        }),
        "Nexus `primary_key` lists one or more column names separated by commas"
    );
    assert_eq!(
        derive_error(parse_quote! {
            #[nexus(primary_key = "")]
            struct LessonProgress { user_id: i32 }
        }),
        "Nexus `primary_key` lists one or more column names separated by commas"
    );
    assert_eq!(
        derive_error(parse_quote! {
            #[nexus(primary_key = "user_id, user_id")]
            struct LessonProgress { user_id: i32 }
        }),
        "Nexus primary-key column `user_id` is listed more than once"
    );
    assert_eq!(
        derive_error(parse_quote! {
            #[nexus(primary_key = "a, b, c, d, e, f, g, h, i")]
            struct Wide { a: i32 }
        }),
        "a Nexus primary key has at most 8 columns"
    );
    let eight: DeriveInput = parse_quote! {
        #[nexus(primary_key = "a, b, c, d, e, f, g, h")]
        struct Wide { a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32 }
    };
    assert!(expand_nexus(&eight).is_ok());
    assert_eq!(
        derive_error(parse_quote! {
            #[nexus(primary_key = "user_id, course_id")]
            struct LessonProgress { user_id: i32, lesson_id: i32 }
        }),
        "Nexus primary key `course_id` is not a field on `LessonProgress`"
    );
    // Annotations must name the listed columns in the listed order.
    assert_eq!(
        derive_error(parse_quote! {
            #[nexus(primary_key = "user_id, lesson_id")]
            struct LessonProgress {
                #[nexus(primary_key)]
                user_id: i32,
                lesson_id: i32,
            }
        }),
        "#[nexus(primary_key)] on `user_id` contradicts the struct-level primary key `user_id, lesson_id`"
    );
    assert_eq!(
        derive_error(parse_quote! {
            #[nexus(primary_key = "user_id")]
            struct LessonProgress {
                #[nexus(primary_key)]
                user_id: i32,
                #[nexus(primary_key)]
                lesson_id: i32,
            }
        }),
        "#[nexus(primary_key)] on `lesson_id` contradicts the struct-level primary key `user_id`"
    );
}
