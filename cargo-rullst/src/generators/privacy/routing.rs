//! Mount the consumer before the router's CSRF layer, or, in a router without
//! one (`Server` applies CSRF since 13.0.0-alpha.6), before its health probe
//! merge, preserving all existing routes and their per-route authorization
//! (including the age gate).
use std::io;
use syn::{Expr, ExprMethodCall, spanned::Spanned, visit::Visit};

#[derive(Default)]
struct Boundary {
    ends: Vec<usize>,
    /// Where `.merge_axum(rullst::health::health_router())` calls start.
    health: Vec<usize>,
    collision: bool,
}
impl<'ast> Visit<'ast> for Boundary {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if node.method == "layer"
            && node.args.len() == 1
            && let Some(Expr::Call(call)) = node.args.first()
            && let Expr::Path(function) = call.func.as_ref()
            && path_is(&function.path, &["rullst", "server", "from_fn"])
            && call.args.len() == 1
            && let Some(Expr::Path(handler)) = call.args.first()
            && path_is(&handler.path, &["rullst", "security", "csrf_middleware"])
        {
            self.ends.push(node.receiver.span().byte_range().end);
        }
        if node.method == "merge_axum"
            && node.args.len() == 1
            && let Some(Expr::Call(call)) = node.args.first()
            && let Expr::Path(function) = call.func.as_ref()
            && path_is(&function.path, &["rullst", "health", "health_router"])
            && call.args.is_empty()
        {
            self.health.push(node.receiver.span().byte_range().end);
        }
        if matches!(
            node.method.to_string().as_str(),
            "route" | "nest" | "nest_axum"
        ) && let Some(Expr::Lit(literal)) = node.args.first()
            && let syn::Lit::Str(path) = &literal.lit
            && privacy_path(&path.value())
        {
            self.collision = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if node
            .path
            .segments
            .iter()
            .any(|part| part.ident == "privacy_controller")
        {
            self.collision = true;
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        if privacy_path(&node.value()) {
            self.collision = true;
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        // syn does not descend into macro arguments. Parse string literals so
        // raw and escaped route names cannot evade the same collision check.
        let mut pending = vec![node.tokens.clone()];
        while let Some(tokens) = pending.pop() {
            for token in tokens {
                match token {
                    proc_macro2::TokenTree::Group(group) => pending.push(group.stream()),
                    proc_macro2::TokenTree::Literal(literal) => {
                        if let Ok(value) = syn::parse_str::<syn::LitStr>(&literal.to_string()) {
                            self.collision |= privacy_path(&value.value());
                        }
                    }
                    proc_macro2::TokenTree::Ident(ident) if ident == "privacy_controller" => {
                        self.collision = true;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn privacy_path(path: &str) -> bool {
    path == "/privacy" || path.starts_with("/privacy/")
}

fn path_is(path: &syn::Path, names: &[&str]) -> bool {
    path.segments
        .iter()
        .map(|part| part.ident.to_string())
        .eq(names.iter().map(|name| (*name).to_owned()))
}

const AMBIGUOUS: &str =
    "privacy routes already exist or the starter's CSRF layer or health probe merge is ambiguous";

pub(super) fn mount(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    mount_if_bounded(source)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, AMBIGUOUS).into())
}

/// [`mount`] for an application's own router: `None` when it has neither a
/// single `rullst::server::from_fn(rullst::security::csrf_middleware)` layer
/// nor, without such layers, a single
/// `.merge_axum(rullst::health::health_router())`, so the developer mounts
/// the routes; existing privacy routes still refuse.
pub(super) fn mount_if_bounded(source: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let parsed = syn::parse_file(source)?;
    let mut boundary = Boundary::default();
    boundary.visit_file(&parsed);
    if boundary.collision {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, AMBIGUOUS).into());
    }
    let anchors = if boundary.ends.is_empty() {
        &boundary.health
    } else {
        &boundary.ends
    };
    let [offset] = anchors.as_slice() else {
        return Ok(None);
    };
    let offset = *offset;
    if !source.is_char_boundary(offset) {
        return Err(io::Error::other("invalid privacy route source boundary").into());
    }
    let mut result = source.to_owned();
    result.insert_str(
        offset,
        ".merge_axum(controllers::privacy_controller::routes().into_axum())",
    );
    syn::parse_file(&result)?;
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_before_csrf_without_changing_authentication_or_existing_age_routes() {
        let source = "// Preferências\nfn router() { let _ = router.route(\"/dashboard\", controllers::age_controller::show).layer(rullst::server::from_fn(rullst::security::csrf_middleware)); }";
        let updated = mount(source).unwrap();
        assert!(updated.contains("controllers::age_controller::show"));
        assert!(
            updated.find("privacy_controller::routes").unwrap()
                < updated.find("csrf_middleware").unwrap()
        );
        assert!(mount(&updated).is_err());
        assert!(mount(&source.replace("csrf_middleware", "other")).is_err());
        // An application's own router without the boundary is left alone,
        // but existing privacy routes still refuse.
        assert_eq!(
            mount_if_bounded(&source.replace("csrf_middleware", "other")).unwrap(),
            None
        );
        assert!(mount_if_bounded(&updated).is_err());
        assert_eq!(
            mount_if_bounded(source).unwrap().as_deref(),
            Some(updated.as_str())
        );
        assert!(mount(&source.replace("/dashboard", "/privacy/export")).is_err());
        // Without a CSRF layer, the routes go before the health probe merge.
        let health = "fn router() { let _ = router.route(\"/dashboard\", show).merge_axum(rullst::health::health_router()).nest_axum(\"/nexus\", nexus); }";
        let mounted = mount(health).unwrap();
        assert_eq!(
            mounted,
            health.replace(
                ".merge_axum(rullst::health::health_router())",
                ".merge_axum(controllers::privacy_controller::routes().into_axum()).merge_axum(rullst::health::health_router())"
            )
        );
        // A CSRF layer wins over the merge; two merges are ambiguous.
        let both = health.replace(
            ".nest_axum",
            ".layer(rullst::server::from_fn(rullst::security::csrf_middleware)).nest_axum",
        );
        let mounted = mount(&both).unwrap();
        assert!(
            mounted.find("privacy_controller::routes").unwrap()
                > mounted.find("health_router").unwrap()
        );
        let twice = health.replace(
            ".nest_axum",
            ".merge_axum(rullst::health::health_router()).nest_axum",
        );
        assert_eq!(mount_if_bounded(&twice).unwrap(), None);
        assert_eq!(
            mount_if_bounded(&health.replace("health_router()", "health_router_with_version(v)"))
                .unwrap(),
            None
        );
        for route in [r#""\x2fprivacy/export""#, r##"r#"/privacy"#"##] {
            let existing =
                format!("{source}\nfn other() {{ routes! {{ GET {route} => handler }}; }}");
            assert!(mount(&existing).is_err());
        }
    }
}
