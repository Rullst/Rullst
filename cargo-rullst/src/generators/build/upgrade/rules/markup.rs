//! `html!` returns `rullst::html::Markup` in v13: functions declared to
//! return a `String` built by `html!`, and `RawHtml` around `html!` output,
//! including inside the `{...}` expressions of an `html!` body.

use super::Collector;
use super::catalog::{HTML_MARKUP_RETURN, HTML_RAW_MARKUP};
use super::rust::mentions;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, Stmt};

/// Scans one parsed file for both rules.
pub(super) fn scan(file: &syn::File, out: &mut Collector) {
    let mut functions = Functions {
        names: BTreeSet::new(),
        out: &mut *out,
    };
    functions.visit_file(file);
    let names = functions.names;
    Wrappers {
        functions: &names,
        bindings: Vec::new(),
        out,
    }
    .visit_file(file);
}

fn line(span: proc_macro2::Span) -> usize {
    span.start().line
}

fn is_html(mac: &syn::Macro) -> bool {
    mac.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "html")
}

/// The value of `block` ends in an `html!` invocation.
fn block_returns_html(block: &syn::Block) -> bool {
    match block.stmts.last() {
        Some(Stmt::Expr(expr, None)) => returns_html(expr),
        Some(Stmt::Macro(statement)) => statement.semi_token.is_none() && is_html(&statement.mac),
        _ => false,
    }
}

/// `expr` evaluates to an `html!` result, possibly through `if`, `match`, a
/// block or an `Ok(..)`, `Some(..)` or `Html(..)` wrapper.
fn returns_html(expr: &Expr) -> bool {
    match expr {
        Expr::Macro(expr) => is_html(&expr.mac),
        Expr::Block(expr) => block_returns_html(&expr.block),
        Expr::If(expr) => {
            block_returns_html(&expr.then_branch)
                || expr
                    .else_branch
                    .as_ref()
                    .is_some_and(|(_, otherwise)| returns_html(otherwise))
        }
        Expr::Match(expr) => expr.arms.iter().any(|arm| returns_html(&arm.body)),
        Expr::Paren(expr) => returns_html(&expr.expr),
        Expr::Return(expr) => expr.expr.as_deref().is_some_and(returns_html),
        Expr::Call(call) => {
            let wrapper = matches!(&*call.func, Expr::Path(path)
            if path.path.segments.last().is_some_and(|segment| {
                matches!(segment.ident.to_string().as_str(), "Ok" | "Some" | "Html")
            }));
            wrapper && call.args.len() == 1 && call.args.first().is_some_and(returns_html)
        }
        _ => false,
    }
}

/// A `return` of an `html!` result in a function body, outside closures and
/// nested items.
#[derive(Default)]
struct Returns {
    found: bool,
}

impl<'ast> Visit<'ast> for Returns {
    fn visit_expr_closure(&mut self, _closure: &'ast syn::ExprClosure) {}

    fn visit_item(&mut self, _item: &'ast syn::Item) {}

    fn visit_expr_return(&mut self, expr: &'ast syn::ExprReturn) {
        self.found |= expr.expr.as_deref().is_some_and(returns_html);
        visit::visit_expr_return(self, expr);
    }
}

/// Functions declared to return a `String` (`String`, `Html<String>`,
/// `Result<String, _>`, …) whose value is an `html!` result.
struct Functions<'a> {
    names: BTreeSet<String>,
    out: &'a mut Collector,
}

impl Functions<'_> {
    fn function(&mut self, signature: &syn::Signature, block: &syn::Block) {
        let syn::ReturnType::Type(_, ty) = &signature.output else {
            return;
        };
        let ty = ty.to_token_stream();
        if !mentions(ty.clone(), "String") || mentions(ty, "Markup") {
            return;
        }
        let mut returns = Returns::default();
        returns.visit_block(block);
        if block_returns_html(block) || returns.found {
            self.names.insert(signature.ident.to_string());
            self.out
                .add(&HTML_MARKUP_RETURN, line(signature.output.span()));
        }
    }
}

impl<'ast> Visit<'ast> for Functions<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        // A native `#[island]` function returns `Markup` in v13.
        let island = item.attrs.iter().any(|attribute| {
            attribute
                .path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "island")
        });
        if island {
            self.names.insert(item.sig.ident.to_string());
        } else {
            self.function(&item.sig, &item.block);
        }
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.function(&item.sig, &item.block);
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if let Some(block) = &item.default {
            self.function(&item.sig, block);
        }
        visit::visit_trait_item_fn(self, item);
    }
}

/// An `html!` invocation anywhere in an expression.
#[derive(Default)]
struct ContainsHtml {
    found: bool,
}

impl<'ast> Visit<'ast> for ContainsHtml {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.found |= is_html(mac);
    }
}

fn contains_html(expr: &Expr) -> bool {
    let mut probe = ContainsHtml::default();
    probe.visit_expr(expr);
    probe.found
}

/// `RawHtml(..)` and `RawHtml::new(..)` around `html!` output: an `html!`
/// invocation, a call of a function found above, of an `#[island]` function
/// or of `Live::mount`, or a local bound to one.
struct Wrappers<'a> {
    functions: &'a BTreeSet<String>,
    /// `let` bindings of `html!` output, per enclosing function.
    bindings: Vec<BTreeSet<String>>,
    out: &'a mut Collector,
}

impl Wrappers<'_> {
    fn is_bound(&self, name: &str) -> bool {
        self.bindings
            .last()
            .is_some_and(|scope| scope.contains(name))
    }

    /// A call of a function found above or of `Live::mount`, which returns
    /// `Markup` in v13.
    fn call_returns_markup(&self, call: &syn::ExprCall) -> bool {
        let Expr::Path(path) = &*call.func else {
            return false;
        };
        let names: Vec<String> = path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect();
        let mounts = matches!(
            names.as_slice(),
            [.., live, mount] if live == "Live" && mount == "mount"
        );
        mounts
            || names
                .last()
                .is_some_and(|name| self.functions.contains(name))
    }

    fn wraps_html(&self, expr: &Expr) -> bool {
        if contains_html(expr) {
            return true;
        }
        match expr {
            Expr::Call(call) => self.call_returns_markup(call),
            Expr::Await(expr) => self.wraps_html(&expr.base),
            Expr::MethodCall(call) => {
                self.functions.contains(&call.method.to_string()) || self.wraps_html(&call.receiver)
            }
            Expr::Path(path) => path
                .path
                .get_ident()
                .is_some_and(|name| self.is_bound(&name.to_string())),
            Expr::Paren(expr) => self.wraps_html(&expr.expr),
            Expr::Reference(expr) => self.wraps_html(&expr.expr),
            _ => false,
        }
    }

    fn scoped(&mut self, visit_body: impl FnOnce(&mut Self)) {
        self.bindings.push(BTreeSet::new());
        visit_body(self);
        self.bindings.pop();
    }

    /// Visits the `{...}` Rust expressions of an `html!` body.
    fn html_body(&mut self, stream: TokenStream) {
        for tree in stream {
            let TokenTree::Group(group) = tree else {
                continue;
            };
            if group.delimiter() == Delimiter::Brace
                && let Ok(expr) = syn::parse2::<Expr>(group.stream())
            {
                self.visit_expr(&expr);
            } else {
                self.html_body(group.stream());
            }
        }
    }
}

fn bound_names(pattern: &syn::Pat, names: &mut BTreeSet<String>) {
    match pattern {
        syn::Pat::Ident(binding) => {
            names.insert(binding.ident.to_string());
        }
        syn::Pat::Type(typed) => bound_names(&typed.pat, names),
        _ => {}
    }
}

impl<'ast> Visit<'ast> for Wrappers<'_> {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.scoped(|this| visit::visit_item_fn(this, item));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.scoped(|this| visit::visit_impl_item_fn(this, item));
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init
            && self.wraps_html(&init.expr)
            && let Some(scope) = self.bindings.last_mut()
        {
            bound_names(&local.pat, scope);
        }
        visit::visit_local(self, local);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let Expr::Path(path) = &*call.func {
            let names: Vec<String> = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            if matches!(names.as_slice(), [.., "RawHtml"] | [.., "RawHtml", "new"])
                && call.args.first().is_some_and(|arg| self.wraps_html(arg))
            {
                self.out.add(&HTML_RAW_MARKUP, line(call.span()));
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if is_html(mac) {
            self.html_body(mac.tokens.clone());
        }
    }
}
