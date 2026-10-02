//! Structural rules over the syntax tree: derive attributes, `impl` blocks,
//! function signatures, `match` arms, struct literals and comparisons.
//! Attributes are read here explicitly; [`super::tokens`] skips them.

use super::catalog::*;
use super::{Collector, Rule, WorkspaceFacts, tokens};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, Fields, Lit, Pat, Token};

/// The file being scanned and the workspace facts some rules need.
pub(super) struct FileContext<'a> {
    pub text: &'a str,
    pub file_name: &'a str,
    pub facts: &'a WorkspaceFacts,
}

/// Per-file facts aggregated per package.
#[derive(Debug, Default)]
pub(super) struct FileSummary {
    /// Line of the first `Server::new`/`Server::new_hot` call.
    pub server_start: Option<usize>,
    /// The file names `health_router` or `health_router_with_lifecycle`.
    pub mounts_health: bool,
}

/// Parses and scans one Rust file.
pub(super) fn scan_source(
    context: &FileContext<'_>,
    out: &mut Collector,
) -> Result<FileSummary, syn::Error> {
    let file = syn::parse_file(context.text)?;
    let mut visitor = Visitor { context, out };
    visitor.visit_file(&file);
    let facts = tokens::scan(file.to_token_stream(), context, visitor.out);
    if facts.protects_router
        && let Some(line) = facts.root_route
    {
        visitor.out.add(&ERP_DASHBOARD_ACCESS, line);
    }
    Ok(FileSummary {
        server_start: facts.server_start,
        mounts_health: facts.mounts_health,
    })
}

fn line(span: proc_macro2::Span) -> usize {
    span.start().line
}

/// Top-level `key` or `key = "value"` entries of `#[name(...)]` attributes.
/// Nested lists are skipped; values that are not string literals are `None`.
fn entries(attributes: &[Attribute], name: &str) -> Vec<(String, Option<String>, usize)> {
    let mut found = Vec::new();
    for attribute in attributes {
        if !attribute.path().is_ident(name) {
            continue;
        }
        let _ = attribute.parse_nested_meta(|meta| {
            let key = meta
                .path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default();
            let at = line(meta.path.span());
            let mut value = None;
            if meta.input.peek(Token![=]) {
                if let Expr::Lit(literal) = meta.value()?.parse::<Expr>()?
                    && let Lit::Str(text) = literal.lit
                {
                    value = Some(text.value());
                }
            } else if meta.input.peek(syn::token::Paren) {
                let _ = meta.input.parse::<proc_macro2::Group>()?;
            }
            found.push((key, value, at));
            Ok(())
        });
    }
    found
}

/// Last path segments of every `#[derive(...)]` entry.
fn derives(attributes: &[Attribute]) -> Vec<String> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("derive"))
        .filter_map(|attribute| {
            attribute
                .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .filter_map(|path| {
            path.segments
                .last()
                .map(|segment| segment.ident.to_string())
        })
        .collect()
}

/// The line of a field's name (its attributes may start earlier).
fn field_line(field: &syn::Field) -> usize {
    field
        .ident
        .as_ref()
        .map_or_else(|| line(field.span()), |name| line(name.span()))
}

fn has_key(entries: &[(String, Option<String>, usize)], keys: &[&str]) -> bool {
    entries
        .iter()
        .any(|(key, _, _)| keys.contains(&key.as_str()))
}

/// Whether the tokens contain the identifier `name`.
fn mentions(tokens: proc_macro2::TokenStream, name: &str) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => ident == name,
        proc_macro2::TokenTree::Group(group) => mentions(group.stream(), name),
        _ => false,
    })
}

struct Visitor<'a, 'b> {
    context: &'a FileContext<'a>,
    out: &'b mut Collector,
}

impl Visitor<'_, '_> {
    fn hit(&mut self, rule: &'static Rule, line: usize) {
        self.out.add(rule, line);
    }

    fn orm_model(&mut self, item: &syn::ItemStruct) {
        for (key, _, at) in entries(&item.attrs, "sqlx") {
            if key != "default" {
                self.hit(&ORM_SQLX_STRUCT_OPTION, at);
            }
        }
        let model = entries(&item.attrs, "orm");
        for (key, _, at) in &model {
            if key == "auditable" {
                self.hit(&AUDIT_PAYLOADS, *at);
            }
        }
        if has_key(&model, &["searchable"]) {
            for (key, value, at) in &model {
                let invalid = value.as_deref().is_some_and(|table| {
                    !(table.starts_with(|c: char| c.is_ascii_lowercase())
                        && table
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
                });
                if matches!(key.as_str(), "table" | "table_name") && invalid {
                    self.hit(&ORM_SEARCHABLE_TABLE, *at);
                }
            }
        }
        for field in &item.fields {
            let options = entries(&field.attrs, "orm");
            let at = field_line(field);
            let relation = options
                .iter()
                .find(|(key, _, _)| {
                    matches!(
                        key.as_str(),
                        "has_many"
                            | "has_one"
                            | "belongs_to"
                            | "belongs_to_many"
                            | "morph_many"
                            | "morph_one"
                            | "morph_to"
                    )
                })
                .map(|(key, _, line)| (key.as_str(), *line));
            match relation {
                Some(("belongs_to", line)) if !has_key(&options, &["foreign_key"]) => {
                    self.hit(&ORM_BELONGS_TO_KEY, line);
                }
                _ => {}
            }
            let ignored = match relation.map(|(key, _)| key) {
                Some("belongs_to" | "morph_to") => Some("local_key"),
                Some("has_one" | "has_many" | "morph_one" | "morph_many") => Some("related_key"),
                _ => None,
            };
            if let Some(ignored) = ignored {
                for (key, _, line) in &options {
                    if key == ignored {
                        self.hit(&ORM_IGNORED_KEY, *line);
                    }
                }
            }
            if has_key(&entries(&field.attrs, "sqlx"), &["json"]) {
                self.hit(&ORM_JSON_SERIALIZE, at);
            }
            if has_key(&options, &["encrypted", "masked"]) {
                self.hit(&PROTECTED_VALUES, at);
            }
            if field
                .ident
                .as_ref()
                .is_some_and(|name| name == "password_hash")
                && !has_key(&options, &["hidden", "skip"])
            {
                self.hit(&PASSWORD_HASH_HIDDEN, at);
            }
        }
    }

    fn nexus_model(&mut self, item: &syn::ItemStruct) {
        let declared = entries(&item.attrs, "nexus")
            .into_iter()
            .find_map(|(key, value, _)| (key == "primary_key").then_some(value).flatten());
        let mut annotated = 0usize;
        for field in &item.fields {
            let annotation = entries(&field.attrs, "nexus")
                .into_iter()
                .find(|(key, value, _)| key == "primary_key" && value.is_none());
            let Some((_, _, at)) = annotation else {
                continue;
            };
            annotated += 1;
            let contradicts = declared
                .as_deref()
                .is_some_and(|declared| field.ident.as_ref().is_some_and(|name| name != declared));
            if annotated > 1 || contradicts {
                self.hit(&NEXUS_PRIMARY_KEY, at);
            }
        }
    }

    fn deserialized(&mut self, item: &syn::ItemStruct) {
        for field in &item.fields {
            let serde = entries(&field.attrs, "serde");
            let guarded = has_key(
                &serde,
                &["deserialize_with", "with", "skip", "skip_deserializing"],
            );
            let at = field_line(field);
            if !guarded && mentions(field.ty.to_token_stream(), "SecretString") {
                self.hit(&SECRET_STRING_INPUT, at);
            }
            if self.context.file_name == "mfa.rs"
                && field.ident.as_ref().is_some_and(|name| name == "secret")
            {
                self.hit(&MFA_CLIENT_SECRET, at);
            }
        }
    }

    /// `SecretString` fields of a serialized struct or a model, whose
    /// projections no longer carry the plaintext.
    fn serialized_secrets(&mut self, item: &syn::ItemStruct) {
        for field in &item.fields {
            if mentions(field.ty.to_token_stream(), "SecretString") {
                self.hit(&PROTECTED_VALUES, field_line(field));
            }
        }
    }

    fn function(&mut self, attributes: &[Attribute], signature: &syn::Signature) {
        for attribute in attributes {
            let segments: Vec<String> = attribute
                .path()
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            if let [.., scope, last] = segments.as_slice()
                && last == "test"
                && matches!(scope.as_str(), "rullst_orm" | "orm")
            {
                self.hit(&ORM_SANDBOX_TEST, line(attribute.span()));
            }
        }
        let name = signature.ident.to_string();
        if matches!(name.as_str(), "index_page" | "detail_page")
            && !signature.inputs.iter().any(|input| {
                matches!(input, syn::FnArg::Typed(typed)
                    if typed.pat.to_token_stream().to_string().contains("nonce"))
            })
        {
            self.hit(&PAGE_CSP_NONCE, line(signature.ident.span()));
        }
    }
}

/// A pattern that matches every value (`_`, a plain binding, `(_, _)`).
fn catches_all(pattern: &Pat) -> bool {
    match pattern {
        Pat::Wild(_) => true,
        Pat::Ident(binding) => binding.subpat.is_none(),
        Pat::Tuple(tuple) => tuple.elems.iter().all(catches_all),
        Pat::Or(alternatives) => alternatives.cases.iter().any(catches_all),
        _ => false,
    }
}

fn is_empty_string(expr: &Expr) -> bool {
    let empty_literal = |expr: &Expr| matches!(expr, Expr::Lit(literal) if matches!(&literal.lit, Lit::Str(text) if text.value().is_empty()));
    match expr {
        Expr::Call(call) => {
            let path = call.func.to_token_stream().to_string().replace(' ', "");
            (path.ends_with("String::new") && call.args.is_empty())
                || (path.ends_with("String::from")
                    && call.args.len() == 1
                    && call.args.first().is_some_and(empty_literal))
        }
        Expr::MethodCall(call) => {
            matches!(
                call.method.to_string().as_str(),
                "to_string" | "into" | "to_owned"
            ) && empty_literal(&call.receiver)
        }
        _ => false,
    }
}

impl<'ast> Visit<'ast> for Visitor<'_, '_> {
    // Attributes are inspected by the item visitors; their literals (doc
    // comments included) must never reach expression rules.
    fn visit_attribute(&mut self, _attribute: &'ast Attribute) {}

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        let derived = derives(&item.attrs);
        let has_derive = |name: &str| derived.iter().any(|derive| derive == name);
        if matches!(item.fields, Fields::Named(_)) {
            if has_derive("Orm") || has_derive("TursoModel") {
                self.orm_model(item);
            }
            if has_derive("Nexus") {
                self.nexus_model(item);
            }
            if has_derive("Deserialize") {
                self.deserialized(item);
            }
            if has_derive("Serialize") || has_derive("Orm") {
                self.serialized_secrets(item);
            }
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if let Some((path, _)) = &item.trait_
            && path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "QueueDriver")
        {
            self.hit(&QUEUE_DRIVER_PREVIEWS, line(path.span()));
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.function(&item.attrs, &item.sig);
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.function(&item.attrs, &item.sig);
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_expr_match(&mut self, expr: &'ast syn::ExprMatch) {
        let names_kind = expr
            .arms
            .iter()
            .any(|arm| mentions(arm.pat.to_token_stream(), "FieldKind"));
        if names_kind && !expr.arms.iter().any(|arm| catches_all(&arm.pat)) {
            self.hit(&NEXUS_FIELD_KIND_MATCH, line(expr.match_token.span()));
        }
        visit::visit_expr_match(self, expr);
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        for field in &expr.fields {
            if let syn::Member::Named(name) = &field.member
                && (name == "created_at" || name == "updated_at")
                && is_empty_string(&field.expr)
            {
                self.hit(&EMPTY_TIMESTAMPS, line(name.span()));
            }
        }
        visit::visit_expr_struct(self, expr);
    }

    fn visit_expr_binary(&mut self, expr: &'ast syn::ExprBinary) {
        let ordering = matches!(
            expr.op,
            syn::BinOp::Lt(_) | syn::BinOp::Le(_) | syn::BinOp::Gt(_) | syn::BinOp::Ge(_)
        );
        let length_of_credential = |side: &Expr| match side {
            Expr::MethodCall(call) if call.method == "len" && call.args.is_empty() => {
                let receiver = call.receiver.to_token_stream().to_string();
                receiver.contains("password") || receiver.contains("name")
            }
            _ => false,
        };
        let registration_limit = |side: &Expr| {
            matches!(side, Expr::Lit(literal)
                if matches!(&literal.lit, Lit::Int(value) if matches!(value.base10_digits(), "12" | "120")))
        };
        if ordering
            && ((length_of_credential(&expr.left) && registration_limit(&expr.right))
                || (length_of_credential(&expr.right) && registration_limit(&expr.left)))
        {
            self.hit(&UTF16_LENGTHS, line(expr.span()));
        }
        visit::visit_expr_binary(self, expr);
    }
}
