//! Every SQLx model a starter writes opts into dirty tracking, so two
//! requests that edit different fields of one row both persist.

use super::*;
use quote::ToTokens;

/// The `#[derive(Orm)]` structs of one file, with whether each declares a
/// `#[sqlx(skip)] pub snapshot: RowSnapshot` field.
fn orm_models(source: &str) -> Vec<(String, bool)> {
    let file = syn::parse_file(source).expect("model parses");
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(model) => Some(model),
            _ => None,
        })
        .filter(|model| {
            model.attrs.iter().any(|attribute| {
                attribute.path().is_ident("derive")
                    && attribute.meta.to_token_stream().to_string().contains("Orm")
            })
        })
        .map(|model| {
            let tracked = model.fields.iter().any(|field| {
                field.ident.as_ref().is_some_and(|name| name == "snapshot")
                    && field.ty.to_token_stream().to_string() == "RowSnapshot"
                    && field.attrs.iter().any(|attribute| {
                        attribute.meta.to_token_stream().to_string() == "sqlx (skip)"
                    })
            });
            (model.ident.to_string(), tracked)
        })
        .collect()
}

#[test]
fn every_starter_sqlx_model_declares_a_row_snapshot() {
    let (orm, frontend) = ("Active Record", "Zero-Bundle HTMX");
    let starters = [
        (
            "blank",
            blank::file_manifest("demo", "demo", false, false, true, orm, frontend),
            vec!["User"],
        ),
        (
            "blog",
            blog::file_manifest("demo", false, orm, frontend),
            vec!["Post"],
        ),
        (
            "erp",
            erp::file_manifest("demo", false, orm, frontend),
            vec!["Order", "Product"],
        ),
        (
            "portfolio",
            portfolio::file_manifest("demo", false, orm, frontend),
            vec!["Experience", "Profile", "Project", "Skill"],
        ),
        (
            "saas",
            saas::file_manifest("demo", false, orm, frontend),
            vec!["BillingCustomer", "Subscription", "User"],
        ),
        (
            "lms",
            lms::file_manifest("demo", false, orm, frontend),
            vec![
                "Category",
                "Course",
                "CourseModule",
                "Enrollment",
                "Lesson",
                "LessonProgress",
                "LessonProgressEvent",
                "User",
            ],
        ),
    ];
    for (starter, manifest, expected) in starters {
        let mut models = Vec::new();
        for (path, source) in &manifest {
            if !path.ends_with(".rs") {
                continue;
            }
            for (name, tracked) in orm_models(source) {
                assert!(tracked, "{starter}: {path} {name} has no RowSnapshot field");
                assert!(
                    source.contains("RowSnapshot}") || source.contains("RowSnapshot,"),
                    "{starter}: {path} does not import RowSnapshot"
                );
                models.push(name);
            }
        }
        models.sort();
        assert_eq!(models, expected, "{starter}");
    }
}
