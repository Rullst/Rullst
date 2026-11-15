//! Replace only the recognized dashboard method router, preserving other source.
//! The age gate's form needs no CSRF layer on the router: `Server` applies
//! CSRF to every route since 13.0.0-alpha.6.
use quote::ToTokens;
use std::{io, ops::Range};
use syn::{Expr, ExprMethodCall, Lit, spanned::Spanned, visit::Visit};

const ORIGINAL: &str = "rullst::routing::get(controllers::auth_controller::dashboard).layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware))";
const REPLACEMENT: &str = "rullst::routing::get(controllers::age_controller::show)\n        .post(controllers::age_controller::submit)\n        .layer(rullst::server::DefaultBodyLimit::max(16 * 1024))\n        .layer(rullst::server::from_fn(controllers::age_controller::with_age_gate))\n        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware))";

struct CallPunctuation;
impl syn::visit_mut::VisitMut for CallPunctuation {
    fn visit_expr_call_mut(&mut self, node: &mut syn::ExprCall) {
        node.args.pop_punct();
        syn::visit_mut::visit_expr_call_mut(self, node);
    }
    fn visit_expr_method_call_mut(&mut self, node: &mut ExprMethodCall) {
        node.args.pop_punct();
        syn::visit_mut::visit_expr_method_call_mut(self, node);
    }
}

pub(super) fn call_tokens(expression: &Expr) -> String {
    use syn::visit_mut::VisitMut;
    let mut expression = expression.clone();
    CallPunctuation.visit_expr_mut(&mut expression);
    expression.into_token_stream().to_string()
}

struct DashboardVisitor {
    expected: String,
    dashboard_routes: usize,
    replacements: Vec<Range<usize>>,
}

impl<'ast> Visit<'ast> for DashboardVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if node.method == "route"
            && let Some(Expr::Lit(path)) = node.args.first()
            && let Lit::Str(path) = &path.lit
            && path.value() == "/dashboard"
        {
            self.dashboard_routes += 1;
            if let Some(router) = node.args.iter().nth(1)
                && node.args.len() == 2
                && call_tokens(router) == self.expected
            {
                self.replacements.push(router.span().byte_range());
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

pub(super) fn protect(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parsed = syn::parse_file(source)?;
    let mut visitor = DashboardVisitor {
        expected: call_tokens(&syn::parse_str::<Expr>(ORIGINAL)?),
        dashboard_routes: 0,
        replacements: Vec::new(),
    };
    visitor.visit_file(&parsed);
    if visitor.dashboard_routes != 1 || visitor.replacements.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unrecognized dashboard route or missing authentication layer",
        )
        .into());
    }
    let range = visitor.replacements.remove(0);
    let mut updated = source.to_owned();
    if updated.get(range.clone()).is_none() {
        return Err(io::Error::other("invalid dashboard source range").into());
    }
    updated.replace_range(range, REPLACEMENT);
    syn::parse_file(&updated)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_and_unicode_do_not_change_the_dashboard_source_boundary() {
        let source = format!(
            "// Declaração — keep this comment\nfn router() {{ let _ = other.route(\"/dashboard\", {ORIGINAL}).layer(rullst::server::from_fn(rullst::security::csrf_middleware)); }}"
        );
        let spaced = source.replace("::", " :: ").replace(".layer", "\n .layer");
        let protected = protect(&spaced).unwrap();
        assert!(protected.starts_with("// Declaração — keep this comment"));
        assert!(protected.contains(REPLACEMENT));
        assert!(
            protect(&source.replace(
                ".layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware))",
                ""
            ))
            .is_err()
        );
        // `Server` applies CSRF, so the router needs no CSRF layer.
        let without_csrf = source.replace(
            ".layer(rullst::server::from_fn(rullst::security::csrf_middleware))",
            "",
        );
        assert!(protect(&without_csrf).unwrap().contains(REPLACEMENT));
        assert!(protect(&source.replace("/dashboard", "/different")).is_err());
    }
}
