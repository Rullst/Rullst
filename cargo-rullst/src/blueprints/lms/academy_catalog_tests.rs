//! Catalog continuation for the materialized SQLite Academy acceptance test.

pub const GENERATED_CATALOG_TESTS_SUFFIX: &str = r##"
        let (catalog_query, catalog_category, catalog_courses) =
            crate::controllers::lms_controller::search_courses(
                &crate::controllers::lms_controller::CatalogQuery {
                    q: Some("Rust".to_string()),
                    category: Some(1),
                },
            )
            .await
            .expect("bounded catalog title search");
        assert_eq!(catalog_query, "Rust");
        assert_eq!(catalog_category, Some(1));
        assert_eq!(catalog_courses.len(), 1);
        assert_eq!(catalog_courses[0].id, 1);

        let (_, _, cross_category_courses) =
            crate::controllers::lms_controller::search_courses(
                &crate::controllers::lms_controller::CatalogQuery {
                    q: Some("Rust".to_string()),
                    category: Some(2),
                },
            )
            .await
            .expect("category-bound catalog search");
        assert!(cross_category_courses.is_empty());

        let (_, _, injection_shaped_courses) =
            crate::controllers::lms_controller::search_courses(
                &crate::controllers::lms_controller::CatalogQuery {
                    q: Some("Rust' OR 1=1 --".to_string()),
                    category: None,
                },
            )
            .await
            .expect("injection-shaped search remains a bound value");
        assert!(injection_shaped_courses.is_empty());
        assert!(matches!(
            crate::controllers::lms_controller::normalize_catalog_query(
                &crate::controllers::lms_controller::CatalogQuery {
                    q: Some("x".repeat(101)),
                    category: None,
                },
            ),
            Err(crate::controllers::lms_controller::CatalogError::InvalidQuery)
        ));

        let catalog_categories = crate::models::category::Category::query()
            .order_by("name")
            .limit(100)
            .get()
            .await
            .expect("catalog categories");
        let rendered_catalog = crate::pages::lms::index_page(
            catalog_categories,
            catalog_courses,
            "<script>alert(1)</script>",
            Some(1),
            "catalog-csp-nonce",
        );
        assert!(rendered_catalog.contains("nonce=\"catalog-csp-nonce\""));
        assert!(rendered_catalog.contains("&lt;script&gt;"));
        assert!(!rendered_catalog.contains("<script>alert(1)</script>"));
        assert!(!rendered_catalog.contains("https://"));
"##;
