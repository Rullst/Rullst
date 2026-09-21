//! Keep the LMS learning router intact while giving the dashboard its own gate.
use std::{io, ops::Range};
use syn::{
    Expr, Ident, LitStr, Local, Path, Token, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::{Pair, Punctuated},
    spanned::Spanned,
    visit::Visit,
};

const AUTH: &str = "rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)";
const DASHBOARD: &str = ".route(\"/dashboard\", rullst::routing::get(controllers::age_controller::show)\n        .post(controllers::age_controller::submit)\n        .layer(rullst::server::DefaultBodyLimit::max(16 * 1024))\n        .layer(rullst::server::from_fn(controllers::age_controller::with_age_gate))\n        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware))\n        .layer(rullst::server::from_fn(controllers::age_controller::select_school)))";

struct RouteEntry {
    method: Ident,
    path: LitStr,
    handler: Path,
    range: Range<usize>,
}

impl Parse for RouteEntry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let method: Ident = input.parse()?;
        let arguments;
        let parentheses = parenthesized!(arguments in input);
        let path: LitStr = arguments.parse()?;
        arguments.parse::<Token![=>]>()?;
        let handler: Path = arguments.parse()?;
        if !arguments.is_empty() {
            return Err(arguments.error("unrecognized LMS route arguments"));
        }
        let range = method.span().byte_range().start..parentheses.span.close().byte_range().end;
        Ok(Self {
            method,
            path,
            handler,
            range,
        })
    }
}

struct Visitor {
    expected_auth: String,
    learning_count: usize,
    ranges: Vec<(Range<usize>, Range<usize>)>,
    csrf: bool,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_local(&mut self, node: &'ast Local) {
        if let syn::Pat::Ident(pattern) = &node.pat
            && pattern.ident == "learning"
        {
            self.learning_count += 1;
            if let Some(init) = &node.init
                && let Expr::MethodCall(layer) = init.expr.as_ref()
                && layer.method == "layer"
                && layer.args.len() == 1
                && layer
                    .args
                    .first()
                    .is_some_and(|arg| super::routing::call_tokens(arg) == self.expected_auth)
                && let Expr::Macro(routes) = layer.receiver.as_ref()
                && routes.mac.path.is_ident("routes")
                && let Ok(entries) = routes
                    .mac
                    .parse_body_with(Punctuated::<RouteEntry, Token![,]>::parse_terminated)
            {
                let dashboards = entries
                    .iter()
                    .filter(|entry| entry.path.value() == "/dashboard")
                    .count();
                if dashboards == 1 {
                    for pair in entries.pairs() {
                        let entry = pair.value();
                        if entry.path.value() != "/dashboard" {
                            continue;
                        }
                        let handler = entry
                            .handler
                            .segments
                            .iter()
                            .map(|part| part.ident.to_string())
                            .collect::<Vec<_>>();
                        if entry.method != "get"
                            || handler != ["controllers", "auth_controller", "dashboard"]
                        {
                            continue;
                        }
                        let mut range = entry.range.clone();
                        if let Pair::Punctuated(_, comma) = pair {
                            range.end = comma.span().byte_range().end;
                        }
                        self.ranges.push((init.expr.span().byte_range(), range));
                    }
                }
            }
        }
        syn::visit::visit_local(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        let path = node
            .path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>();
        if path == ["rullst", "security", "csrf_middleware"] {
            self.csrf = true;
        }
        syn::visit::visit_expr_path(self, node);
    }
}

pub(super) fn protect(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parsed = syn::parse_file(source)?;
    let mut visitor = Visitor {
        expected_auth: super::routing::call_tokens(&syn::parse_str(AUTH)?),
        learning_count: 0,
        ranges: Vec::new(),
        csrf: false,
    };
    visitor.visit_file(&parsed);
    if visitor.learning_count != 1 || visitor.ranges.len() != 1 || !visitor.csrf {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unrecognized LMS learning router or missing authentication/CSRF layer",
        )
        .into());
    }
    let (initializer, removed) = visitor.ranges.remove(0);
    if removed.start < initializer.start || removed.end > initializer.end {
        return Err(io::Error::other("invalid LMS dashboard source range").into());
    }
    let prefix = source
        .get(initializer.start..removed.start)
        .ok_or_else(|| io::Error::other("invalid LMS source prefix"))?;
    // Removing a whole route must not leave its indentation on an empty line.
    // Only this AST-bounded gap is whitespace; never trim application literals.
    let prefix = if prefix
        .rsplit_once('\n')
        .is_some_and(|(_, tail)| tail.chars().all(|c| c == ' ' || c == '\t'))
    {
        prefix.trim_end_matches([' ', '\t'])
    } else {
        prefix
    };
    let suffix = source
        .get(removed.end..initializer.end)
        .ok_or_else(|| io::Error::other("invalid LMS source suffix"))?;
    let mut updated = source.to_owned();
    updated.replace_range(initializer, &format!("{prefix}{suffix}\n    {DASHBOARD}"));
    syn::parse_file(&updated)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lms_dashboard_is_separate_from_existing_learning_authentication() {
        let source = format!(
            r#"// Escola — preserve comment
fn router() {{
    let learning = routes![
        get("/dashboard" => controllers::auth_controller::dashboard),
        // Keep the enrollment route and its original layer.
        post("/courses/{{id}}/enroll" => controllers::learning_controller::enroll),
    ].layer({AUTH});
    let _ = learning.layer(rullst::server::from_fn(rullst::security::csrf_middleware));
}}"#
        );
        let protected = protect(&source).unwrap();
        assert!(protected.starts_with("// Escola — preserve comment"));
        assert!(protected.contains(
            "post(\"/courses/{id}/enroll\" => controllers::learning_controller::enroll)"
        ));
        assert!(
            !protected.contains("get(\"/dashboard\" => controllers::auth_controller::dashboard)")
        );
        assert!(protected.contains(DASHBOARD));
        assert!(
            !protected
                .lines()
                .any(|line| !line.is_empty() && line.trim().is_empty())
        );
        assert!(
            protect(&source.replace("auth_middleware::auth_middleware", "auth_middleware::other"))
                .is_err()
        );
        assert!(protect(&source.replace("csrf_middleware", "other")).is_err());
        assert!(protect(&source.replace("get(\"/dashboard\"", "post(\"/dashboard\"")).is_err());
    }
}
