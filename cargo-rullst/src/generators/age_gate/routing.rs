//! Replace only the recognized dashboard method router, preserving other source.
use quote::ToTokens;
use std::{
    io::{self, Write},
    ops::Range,
    process::{Command, Stdio},
};
use syn::{Expr, ExprMethodCall, Lit, spanned::Spanned, visit::Visit};

const ORIGINAL: &str = "rullst::routing::get(controllers::auth_controller::dashboard).layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware))";
const REPLACEMENT: &str = "rullst::routing::get(controllers::age_controller::show)\n        .post(controllers::age_controller::submit)\n        .layer(rullst::server::DefaultBodyLimit::max(16 * 1024))\n        .layer(rullst::server::from_fn(controllers::age_controller::with_age_gate))\n        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware))";

pub(super) fn equivalent(left: &str, right: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let left = syn::parse_file(left)?.into_token_stream().to_string();
    let right = syn::parse_file(right)?.into_token_stream().to_string();
    if left == right {
        return Ok(true);
    }
    // Rustfmt may add trailing punctuation. Normalize both parsed token streams
    // with the same installed formatter instead of ignoring semantic AST nodes.
    Ok(formatted(&left)? == formatted(&right)?)
}

pub(super) fn formatted(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new("rustfmt")
        .args([
            "--edition",
            "2024",
            "--emit",
            "stdout",
            "--config",
            "skip_children=true",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let write = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("rustfmt stdin unavailable"))?
        .write_all(source.as_bytes());
    let output = child.wait_with_output()?;
    write?;
    if !output.status.success() {
        return Err(io::Error::other("rustfmt could not normalize the SaaS source").into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

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
    csrf: bool,
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

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        let path = node
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if path == ["rullst", "security", "csrf_middleware"] {
            self.csrf = true;
        }
        syn::visit::visit_expr_path(self, node);
    }
}

pub(super) fn protect(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parsed = syn::parse_file(source)?;
    let mut visitor = DashboardVisitor {
        expected: call_tokens(&syn::parse_str::<Expr>(ORIGINAL)?),
        dashboard_routes: 0,
        replacements: Vec::new(),
        csrf: false,
    };
    visitor.visit_file(&parsed);
    if visitor.dashboard_routes != 1 || visitor.replacements.len() != 1 || !visitor.csrf {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unrecognized dashboard route or missing authentication/CSRF layer",
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
        assert!(equivalent("// comment\nfn one() {}", "fn one ( ) { }").unwrap());
        assert!(!equivalent("fn one() {}", "fn two() {}").unwrap());
        assert!(
            protect(&source.replace(
                ".layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware))",
                ""
            ))
            .is_err()
        );
        assert!(protect(&source.replace("csrf_middleware", "other_middleware")).is_err());
        assert!(protect(&source.replace("/dashboard", "/different")).is_err());
    }
}
