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

        let rendered_video = crate::pages::lms::lesson_player_page(
            "Memory safety <essentials>",
            "video",
            "https://media.example.test/lesson.webm",
            "/static/media/lesson.en.vtt",
            "Never render <script>alert('transcript')</script> as markup.",
            "en",
            1,
            1,
            50,
            "csrf-token",
            "progress-key",
            "player-csp-nonce",
        )
        .expect("bounded accessible video player");
        assert!(rendered_video.contains("<video"));
        assert!(rendered_video.contains("kind=\"captions\""));
        assert!(rendered_video.contains("default"));
        assert!(rendered_video.contains("nonce=\"player-csp-nonce\""));
        assert!(rendered_video.contains("&lt;script&gt;"));
        assert!(!rendered_video.contains("<script>alert('transcript')</script>"));
        assert!(!rendered_video.contains("autoplay"));

        let rendered_audio = crate::pages::lms::lesson_player_page(
            "Listening exercise",
            "audio",
            "/static/media/exercise.ogg",
            "",
            "An accessible listening exercise transcript.",
            "pt-BR",
            1,
            2,
            0,
            "csrf-token",
            "progress-key-audio",
            "player-csp-nonce",
        )
        .expect("bounded accessible audio player");
        assert!(rendered_audio.contains("<audio"));
        assert!(rendered_audio.contains("Transcript (pt-BR)"));

        for unsafe_source in [
            "http://media.example.test/lesson.webm",
            "javascript:alert(1)",
            "//media.example.test/lesson.webm",
            "/static/media/lesson\\evil.webm",
        ] {
            assert!(matches!(
                crate::pages::lms::lesson_player_page(
                    "Unsafe source",
                    "audio",
                    unsafe_source,
                    "",
                    "Bounded transcript.",
                    "en",
                    1,
                    2,
                    0,
                    "csrf-token",
                    "progress-key-unsafe",
                    "player-csp-nonce",
                ),
                Err(crate::pages::lms::LessonMediaError::InvalidSource)
            ));
        }
        assert!(matches!(
            crate::pages::lms::lesson_player_page(
                "Missing captions",
                "video",
                "/static/media/lesson.webm",
                "",
                "Bounded transcript.",
                "en",
                1,
                1,
                0,
                "csrf-token",
                "progress-key-captions",
                "player-csp-nonce",
            ),
            Err(crate::pages::lms::LessonMediaError::MissingCaptions)
        ));
        assert!(matches!(
            crate::pages::lms::lesson_player_page(
                "Invalid language",
                "audio",
                "/static/media/lesson.ogg",
                "",
                "Bounded transcript.",
                "en_US",
                1,
                1,
                0,
                "csrf-token",
                "progress-key-language",
                "player-csp-nonce",
            ),
            Err(crate::pages::lms::LessonMediaError::InvalidLanguage)
        ));
        assert!(matches!(
            crate::pages::lms::lesson_player_page(
                "Missing transcript",
                "audio",
                "/static/media/lesson.ogg",
                "",
                "",
                "en",
                1,
                1,
                0,
                "csrf-token",
                "progress-key-transcript",
                "player-csp-nonce",
            ),
            Err(crate::pages::lms::LessonMediaError::InvalidTranscript)
        ));
        assert!(matches!(
            crate::pages::lms::lesson_player_page(
                "Unknown media",
                "stream",
                "/static/media/lesson.bin",
                "",
                "Bounded transcript.",
                "en",
                1,
                1,
                0,
                "csrf-token",
                "progress-key-kind",
                "player-csp-nonce",
            ),
            Err(crate::pages::lms::LessonMediaError::InvalidKind)
        ));
"##;
