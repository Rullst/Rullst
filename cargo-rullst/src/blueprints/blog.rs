// src/blueprints/blog.rs — Blog / Content System blueprint: posts read by slug, a
// paged index, robots.txt and sitemap.xml, with a Nexus CMS.
// The templates under `blog/src/` mirror the generated project's layout.
use super::common;

/// Files every blog variant emits unchanged, in manifest order.
const COMMON_FILES: [(&str, &str); 6] = [
    (
        "src/migrations/m20260601000000_create_posts_table.rs",
        include_str!("blog/src/migrations/m20260601000000_create_posts_table.rs.template"),
    ),
    (
        "src/migrations/mod.rs",
        include_str!("blog/src/migrations/mod.rs.template"),
    ),
    (
        "src/models/post.rs",
        include_str!("blog/src/models/post.rs.template"),
    ),
    (
        "src/models/mod.rs",
        include_str!("blog/src/models/mod.rs.template"),
    ),
    // Reads posts through bounded, ordered queries: the index pages through the
    // newest posts and a post is looked up by its slug, so neither depends on
    // the ORM's default row cap. Database failures return `503`, not `404`.
    // `robots.txt` and `sitemap.xml` use absolute URLs from `RULLST_PUBLIC_ORIGIN`.
    (
        "src/controllers/blog_controller.rs",
        include_str!("blog/src/controllers/blog_controller.rs.template"),
    ),
    (
        "src/controllers/mod.rs",
        include_str!("blog/src/controllers/mod.rs.template"),
    ),
];

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let repo_mod_decl = common::repo_mod_decl(orm_pattern);
    let mut manifest = Vec::new();

    if hot_reload {
        manifest.push((
            "src/lib.rs",
            include_str!("blog/src/lib.rs.template").replace("__REPO_MOD_DECL__", repo_mod_decl),
        ));
        // The caller-supplied name is substituted last so it is never re-expanded.
        manifest.push((
            "src/main.rs",
            include_str!("blog/src/main.hot.rs.template")
                .replace("__REPO_MOD_DECL__", repo_mod_decl)
                .replace("__PROJECT_NAME_SAFE__", project_name_safe),
        ));
    } else {
        manifest.push((
            "src/main.rs",
            include_str!("blog/src/main.rs.template").replace("__REPO_MOD_DECL__", repo_mod_decl),
        ));
        manifest.push((
            super::security_tests::PATH,
            super::security_tests::source(super::security_tests::Starter::Blog),
        ));
    }

    manifest.extend(
        COMMON_FILES
            .iter()
            .map(|(path, template)| (*path, (*template).to_string())),
    );

    if common::is_repo_mode(orm_pattern) {
        manifest.push((
            "src/repositories/post_repository.rs",
            common::generate_repository("Post", "posts"),
        ));
        manifest.push((
            "src/repositories/mod.rs",
            common::generate_repositories_mod(&["Post"]),
        ));
    }

    manifest.push((
        "src/pages/blog.rs",
        include_str!("blog/src/pages/blog.rs.template").replace(
            "__FE_IMPORTS__",
            &common::frontend_page_imports(frontend_engine),
        ),
    ));
    manifest.push((
        "src/pages/mod.rs",
        include_str!("blog/src/pages/mod.rs.template").to_string(),
    ));

    manifest
}

#[cfg(test)]
mod tests {
    use super::file_manifest;

    fn source<'a>(manifest: &'a [(&'static str, String)], name: &str) -> &'a str {
        manifest
            .iter()
            .find_map(|(path, source)| (*path == name).then_some(source.as_str()))
            .unwrap_or_default()
    }

    #[test]
    fn posts_are_read_by_slug_and_paged_without_the_row_cap() {
        for orm_pattern in ["Active Record", "Repository"] {
            let manifest = file_manifest("blog_app", false, orm_pattern, "Zero-Bundle HTMX");
            let controller = source(&manifest, "src/controllers/blog_controller.rs");
            let page = source(&manifest, "src/pages/blog.rs");
            // `all()` stops at the ORM's 1000-row cap without ORDER BY, and
            // `unwrap_or_default` turned a database outage into a 404.
            assert!(!controller.contains("::all()"));
            assert!(!controller.contains("find_all()"));
            assert!(controller.contains("Post::query().where_eq(\"slug\", slug).first().await"));
            assert!(controller.contains(".order_by_desc(\"id\").paginate(page, POSTS_PER_PAGE)"));
            assert_eq!(
                controller
                    .matches("Err(error) => unavailable(error)")
                    .count(),
                2
            );
            assert!(page.contains("posts: rullst_orm::PaginationResult<Post>"));
            assert!(page.contains("href={format!(\"/?page={page}\")}"));
            // An unbounded `?page=` overflowed the OFFSET and became a 503.
            assert!(controller.contains(
                "if page > MAX_PAGE {\n        return (StatusCode::NOT_FOUND, \"Page not found\").into_response();"
            ));
        }
    }

    #[test]
    fn robots_and_sitemap_advertise_only_absolute_urls() {
        let manifest = file_manifest("blog_app", false, "Active Record", "Zero-Bundle HTMX");
        let controller = source(&manifest, "src/controllers/blog_controller.rs");
        // Crawlers reject `<loc>/</loc>` and a relative `Sitemap:` line, and
        // the sitemap listed none of the posts.
        assert!(!controller.contains("<loc>/</loc>"));
        assert!(!controller.contains("Sitemap: /sitemap.xml"));
        assert!(controller.contains("project_setting(\"RULLST_PUBLIC_ORIGIN\")"));
        assert!(controller.contains("format!(\"Sitemap: {origin}/sitemap.xml\\n\")"));
        assert!(controller.contains("<url><loc>{origin}/</loc></url>"));
        assert!(
            controller.contains("<url><loc>{origin}/posts/{}</loc></url>\", path_segment(&slug)")
        );
        assert!(controller.contains("\"SELECT slug FROM posts ORDER BY id DESC LIMIT 1000\""));
        assert!(controller.contains("fn sitemap_urls_are_absolute_encoded_and_xml_safe()"));

        let root = tempfile::tempdir().expect("temporary project");
        crate::generators::project::env_config::generate_env_and_configs(
            root.path(),
            true,
            "Sqlite",
            &[],
            crate::blueprints::BLOG_BLUEPRINT_ID,
            "0123456789abcdef0123456789abcdef",
        )
        .expect("environment scaffold");
        for filename in [".env", ".env.example"] {
            let generated = std::fs::read_to_string(root.path().join(filename))
                .expect("generated environment file");
            assert!(
                generated.contains("\nRULLST_PUBLIC_ORIGIN=\n"),
                "{filename}"
            );
        }
    }
}
