use super::{overlaps, path_matches};

#[test]
fn a_placeholder_matches_exactly_one_ordinary_segment() {
    for (pattern, path) in [
        ("/api/posts/{id}", "/api/posts/7"),
        ("/api/posts/{id}", "/api/posts/a.b"),
        ("/api/{tenant}/posts/{id}", "/api/acme/posts/7"),
        ("/billing/reconcile", "/billing/reconcile"),
        ("/", "/"),
    ] {
        assert!(path_matches(pattern, path), "{pattern} {path}");
    }
    for (pattern, path) in [
        ("/api/posts/{id}", "/api/posts/"),
        ("/api/posts/{id}", "/api/posts/."),
        ("/api/posts/{id}", "/api/posts/.."),
        ("/api/posts/{id}", "/api/posts/7/8"),
        ("/api/posts/{id}", "/api/posts"),
        ("/api/posts/{id}", "/api/users/7"),
        ("/api/{tenant}/posts/{id}", "/api/acme/users/7"),
        ("/billing/reconcile", "/billing/reconcile/"),
        ("/billing/reconcile", "/billing"),
    ] {
        assert!(!path_matches(pattern, path), "{pattern} {path}");
    }
}

#[test]
fn routes_overlap_when_one_path_can_match_both() {
    assert!(overlaps("/a/{id}", "/a/b"));
    assert!(overlaps("/a/b", "/a/{id}"));
    assert!(overlaps("/a/{x}/c", "/a/b/{y}"));
    assert!(overlaps("/a/b", "/a/b"));
    assert!(!overlaps("/a/{id}", "/a/{id}/c"));
    assert!(!overlaps("/a/{id}", "/b/{id}"));
    assert!(!overlaps("/a/{x}/c", "/a/{y}/d"));
}
