//! Extract actual Rust syntax; only erased specification support is introduced.
use quote::ToTokens;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use syn::{Attribute, ImplItem, Item, ItemEnum};

pub struct Projection {
    pub cases: Vec<(&'static str, String)>,
    pub receipt: serde_json::Value,
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}
fn attributes(attrs: &[Attribute], derives: &[&str]) -> Result<(), &'static str> {
    let mut actual = BTreeSet::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            continue;
        }
        if attr.path().is_ident("serde") {
            let text = tokens(attr);
            if ![
                "# [serde (rename_all = \"snake_case\")]",
                "# [serde (deny_unknown_fields)]",
            ]
            .contains(&text.as_str())
            {
                return Err("unreviewed serialization attribute");
            }
            continue;
        }
        if !attr.path().is_ident("derive") {
            return Err("unreviewed type macro or attribute");
        }
        attr.parse_nested_meta(|meta| {
            actual.insert(tokens(&meta.path));
            Ok(())
        })
        .map_err(|_| "invalid derive")?;
    }
    if actual != derives.iter().map(|name| name.to_string()).collect() {
        return Err("unreviewed structural equality derives");
    }
    Ok(())
}
fn enumeration(mut value: ItemEnum, expected: &[&str]) -> Result<String, &'static str> {
    attributes(
        &value.attrs,
        &[
            "Debug",
            "Clone",
            "Copy",
            "PartialEq",
            "Eq",
            "Serialize",
            "Deserialize",
        ],
    )?;
    if !matches!(value.vis, syn::Visibility::Public(_)) || !value.generics.params.is_empty() {
        return Err("unreviewed enum signature");
    }
    let names = value
        .variants
        .iter()
        .map(|v| v.ident.to_string())
        .collect::<Vec<_>>();
    if names != expected {
        return Err("unreviewed risk/method input domain");
    }
    for variant in &mut value.variants {
        if !matches!(variant.fields, syn::Fields::Unit)
            || variant.discriminant.is_some()
            || variant.attrs.iter().any(|a| !a.path().is_ident("doc"))
        {
            return Err("unreviewed enum layout");
        }
        variant.attrs.clear();
    }
    value.attrs.clear();
    // Structural links Verus's equality to the reviewed Rust-derived equality
    // for these exact fieldless enum variants. It adds no executable body.
    Ok(format!(
        "#[derive(PartialEq, Eq, Structural)]\n{}",
        tokens(&value)
    ))
}
pub fn project(source: &str, specification: &str) -> Result<Projection, &'static str> {
    if source.len() > 65536 || specification.len() > 8192 {
        return Err("source budget");
    }
    let parsed = syn::parse_file(source).map_err(|_| "invalid production Rust")?;
    if parsed.attrs.iter().any(|a| !a.path().is_ident("doc")) {
        return Err("unreviewed production file attribute");
    }
    let imports = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Use(value) => Some(tokens(value)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let expected_imports: [syn::ItemUse; 2] = [
        syn::parse_quote!(
            use super::{AgeError, valid_token};
        ),
        syn::parse_quote!(
            use serde::{Deserialize, Serialize};
        ),
    ];
    if imports != expected_imports.iter().map(tokens).collect()
        || parsed.items.iter().any(|item| {
            !matches!(
                item,
                Item::Use(_) | Item::Enum(_) | Item::Struct(_) | Item::Impl(_)
            )
        })
    {
        return Err("unreviewed name resolution or module-level macro");
    }
    let mut types = Vec::new();
    let mut method = None;
    let mut seen = BTreeSet::new();
    for item in parsed.items {
        match item {
            Item::Use(_) => {}
            Item::Enum(value) if value.ident == "RiskLevel" => {
                if !seen.insert("RiskLevel") {
                    return Err("duplicate risk enum");
                }
                types.push(enumeration(value, &["Low", "Elevated", "Restricted"])?);
            }
            Item::Enum(value) if value.ident == "AgeMethod" => {
                if !seen.insert("AgeMethod") {
                    return Err("duplicate method enum");
                }
                types.push(enumeration(
                    value,
                    &["SelfDeclaration", "FacialEstimation", "VerifiedAttribute"],
                )?);
            }
            Item::Struct(mut value) if value.ident == "AgePolicy" => {
                if !seen.insert("AgePolicy") {
                    return Err("duplicate policy struct");
                }
                attributes(
                    &value.attrs,
                    &[
                        "Debug",
                        "Clone",
                        "PartialEq",
                        "Eq",
                        "Serialize",
                        "Deserialize",
                    ],
                )?;
                if !matches!(value.vis, syn::Visibility::Public(_))
                    || !value.generics.params.is_empty()
                {
                    return Err("unreviewed policy signature");
                }
                let fields = value
                    .fields
                    .iter()
                    .map(|f| {
                        (
                            f.ident
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_default(),
                            tokens(&f.ty),
                        )
                    })
                    .collect::<Vec<_>>();
                if fields
                    != [
                        ("version", "String"),
                        ("risk", "RiskLevel"),
                        ("minimum_age", "u8"),
                        ("estimation_margin", "u8"),
                        ("validity_seconds", "u16"),
                    ]
                    .map(|(a, b)| (a.into(), b.into()))
                {
                    return Err("unreviewed policy field domain");
                }
                if value
                    .fields
                    .iter()
                    .any(|f| !f.attrs.is_empty() || !matches!(f.vis, syn::Visibility::Inherited))
                {
                    return Err("unreviewed field visibility or attribute");
                }
                value.attrs.clear();
                types.push(tokens(&value));
            }
            Item::Impl(value) if tokens(&value.self_ty) == "AgePolicy" => {
                if value.trait_.is_some() {
                    return Err("unreviewed policy trait implementation");
                }
                if value.attrs.iter().any(|a| !a.path().is_ident("doc"))
                    || !value.generics.params.is_empty()
                {
                    return Err("unreviewed impl");
                }
                for item in value.items {
                    if let ImplItem::Fn(function) = item
                        && function.sig.ident == "permits"
                    {
                        if method.is_some()
                            || !matches!(function.vis, syn::Visibility::Public(_))
                            || function.attrs.iter().any(|a| !a.path().is_ident("doc"))
                        {
                            return Err("unreviewed decision method");
                        }
                        let expected: syn::Signature =
                            syn::parse_quote!(fn permits(&self, method: AgeMethod) -> bool);
                        if tokens(&function.sig) != tokens(&expected) {
                            return Err("unreviewed decision signature");
                        }
                        method = Some(function);
                    }
                }
            }
            _ => return Err("unreviewed production declaration"),
        }
    }
    if seen.len() != 3 {
        return Err("missing production types");
    }
    let method = method.ok_or("missing production method")?;
    // A newly introduced call must bring its actual implementation into the
    // reviewed proof closure; it cannot resolve to proof-only support instead.
    struct PureBody(bool);
    impl<'a> syn::visit::Visit<'a> for PureBody {
        fn visit_expr_call(&mut self, _: &'a syn::ExprCall) {
            self.0 = false;
        }
        fn visit_expr_method_call(&mut self, _: &'a syn::ExprMethodCall) {
            self.0 = false;
        }
        fn visit_expr_macro(&mut self, _: &'a syn::ExprMacro) {
            self.0 = false;
        }
        fn visit_expr_unsafe(&mut self, _: &'a syn::ExprUnsafe) {
            self.0 = false;
        }
        fn visit_attribute(&mut self, _: &'a Attribute) {
            self.0 = false;
        }
    }
    let mut pure = PureBody(true);
    syn::visit::Visit::visit_block(&mut pure, &method.block);
    if !pure.0 {
        return Err("decision call/macro/unsafe closure requires review");
    }
    let original = tokens(&method.block);
    // The projection retains the actual body. Its only extra statements are
    // specification-only accessor/reveal support, erased before Rust execution.
    let render = |body: &str| {
        format!(
            "use vstd::prelude::*;\nverus! {{\n{}\nimpl AgePolicy {{\n pub closed spec fn risk_spec(&self) -> RiskLevel {{ self.risk }}\n pub fn permits(&self, method: AgeMethod) -> (allowed: bool)\n{specification}\n {{\n proof {{ reveal(AgePolicy::risk_spec); }}\n // BEGIN_LINKED_EXECUTABLE_BODY\n {body}\n // END_LINKED_EXECUTABLE_BODY\n }}\n}}\n}}\nfn main() {{}}\n",
            types.join("\n")
        )
    };
    let mut cases = vec![("production", render(&original))];
    for (label, variant, replacement) in [
        ("restricted_weakening", "Restricted", true),
        ("elevated_weakening", "Elevated", true),
        ("low_denial", "Low", false),
    ] {
        let mut changed = method.block.clone();
        let Some(syn::Stmt::Expr(syn::Expr::Match(expression), _)) = changed.stmts.last_mut()
        else {
            return Err("negative controls need review after decision shape change");
        };
        let arm = expression
            .arms
            .iter_mut()
            .find(|arm| tokens(&arm.pat) == format!("RiskLevel :: {variant}"))
            .ok_or("negative-control arm missing")?;
        let literal = syn::LitBool::new(replacement, proc_macro2::Span::call_site());
        *arm.body = syn::parse_quote!(#literal);
        let mutated = tokens(&changed);
        if mutated == original {
            return Err("negative control already present in production");
        }
        cases.push((label, render(&mutated)));
    }
    let case_records=cases.iter().map(|(name,code)| serde_json::json!({"name":name,"file":format!("{name}.rs"),"sha256":hash(code.as_bytes())})).collect::<Vec<_>>();
    let receipt = serde_json::json!({
        "schema":"rullst.verus-linkage.v1", "production_file":"rullst-privacy/src/age_assurance/policy.rs",
        "entry_point":"AgePolicy::permits", "source_sha256":hash(source.as_bytes()),
        "body_sha256":hash(original.as_bytes()), "types_sha256":hash(types.join("\n").as_bytes()),
        "specification_sha256":hash(specification.as_bytes()), "cases":case_records,
        "runtime_rewrites":[], "added_proof_support":["Structural for exact fieldless derived-equality enums","closed risk accessor","erased reveal step"],
    });
    Ok(Projection { cases, receipt })
}

pub fn module_linkage(lib: &str, age: &str) -> Result<serde_json::Value, &'static str> {
    let lib = syn::parse_file(lib).map_err(|_| "invalid crate root")?;
    let age_file = syn::parse_file(age).map_err(|_| "invalid age module")?;
    let root_module = lib
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Mod(m) = item {
                (m.ident == "age_assurance").then_some(m)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if root_module.len() != 1 {
        return Err("missing or ambiguous age module");
    }
    let module = root_module[0];
    if module.content.is_some()
        || !matches!(module.vis, syn::Visibility::Public(_))
        || module.attrs.len() != 1
        || tokens(&module.attrs[0]) != "# [cfg (feature = \"age-assurance\")]"
    {
        return Err("unreviewed age module path/feature");
    }
    let policy_module = age_file
        .items
        .iter()
        .filter_map(|item| {
            if let Item::Mod(m) = item {
                (m.ident == "policy").then_some(m)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if policy_module.len() != 1
        || policy_module[0].content.is_some()
        || !policy_module[0].attrs.is_empty()
    {
        return Err("unreviewed policy module path");
    }
    let expected: syn::ItemUse = syn::parse_quote!(
        pub use policy::{AgeMethod, AgePolicy, RiskLevel};
    );
    if !age_file
        .items
        .iter()
        .any(|item| matches!(item,Item::Use(u) if tokens(u)==tokens(&expected)))
    {
        return Err("unreviewed public policy exports");
    }
    Ok(
        serde_json::json!({"crate_root_module":tokens(module),"policy_module":tokens(policy_module[0]),"public_exports":tokens(&expected)}),
    )
}
