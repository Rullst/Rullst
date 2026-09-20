//! Mount the consumer before the starter's existing CSRF layer, preserving all
//! existing routes and their per-route authorization (including the age gate).
use std::io;
use syn::{Expr, ExprMethodCall, spanned::Spanned, visit::Visit};

#[derive(Default)]
struct Boundary {
    ends: Vec<usize>,
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
        if matches!(
            node.method.to_string().as_str(),
            "route" | "nest" | "nest_axum"
        ) && let Some(Expr::Lit(literal)) = node.args.first()
            && let syn::Lit::Str(path) = &literal.lit
            && supervision_path(&path.value())
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
            .any(|part| part.ident == "supervision_controller")
        {
            self.collision = true;
        }
        syn::visit::visit_expr_path(self, node);
    }

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        if supervision_path(&node.value()) {
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
                            self.collision |= supervision_path(&value.value());
                        }
                    }
                    proc_macro2::TokenTree::Ident(ident) if ident == "supervision_controller" => {
                        self.collision = true;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn supervision_path(path: &str) -> bool {
    path == "/supervision" || path.starts_with("/supervision/")
}

fn path_is(path: &syn::Path, names: &[&str]) -> bool {
    path.segments
        .iter()
        .map(|part| part.ident.to_string())
        .eq(names.iter().map(|name| (*name).to_owned()))
}

pub(super) fn mount(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let parsed = syn::parse_file(source)?;
    let mut boundary = Boundary::default();
    boundary.visit_file(&parsed);
    if boundary.ends.len() != 1 || boundary.collision {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "supervision routes already exist or the starter CSRF boundary is ambiguous",
        )
        .into());
    }
    let offset = boundary.ends[0];
    if !source.is_char_boundary(offset) {
        return Err(io::Error::other("invalid supervision route source boundary").into());
    }
    let mut result = source.to_owned();
    result.insert_str(
        offset,
        ".merge_axum(controllers::supervision_controller::routes().into_axum())",
    );
    syn::parse_file(&result)?;
    Ok(result)
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
            updated.find("supervision_controller::routes").unwrap()
                < updated.find("csrf_middleware").unwrap()
        );
        assert!(mount(&updated).is_err());
        assert!(mount(&source.replace("csrf_middleware", "other")).is_err());
        assert!(mount(&source.replace("/dashboard", "/supervision/export")).is_err());
        for route in [r#""\x2fsupervision/export""#, r##"r#"/supervision"#"##] {
            let existing =
                format!("{source}\nfn other() {{ routes! {{ GET {route} => handler }}; }}");
            assert!(mount(&existing).is_err());
        }
    }
}
