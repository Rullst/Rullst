//! Mounts `crate::pwa::router()` next to the health probes and declares the
//! `pwa` module in the crate root.
//!
//! Every HTML starter merges `rullst::health::health_router()` into its
//! router exactly once; the PWA routes are merged right after it, outside the
//! CSRF layer (they answer GET only) and inside any later header layer. The
//! merge may be in any module of the crate; without exactly one, the routes
//! are left for the developer to mount and the report says how, unless
//! `--router` names the file to mount them in.
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::{error::Error, io};
use syn::spanned::Spanned;
use syn::visit::Visit;

const HEALTH: &str = "rullst::health::health_router()";

fn compact(stream: &TokenStream) -> String {
    stream.to_string().split_whitespace().collect()
}

/// Where `.merge_axum(rullst::health::health_router())` ends in `source`,
/// and whether the PWA router is mounted. A merge inside a `return`
/// expression builds a fallback router (an application generated from the
/// former ERP starter returns only the probes when its credentials are
/// missing) and is not counted.
fn scan(trees: &[TokenTree], in_return: bool, ends: &mut Vec<usize>, mounted: &mut bool) {
    let mut returning = in_return;
    for (index, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Ident(ident) if ident == "return" => returning = true,
            TokenTree::Punct(punct) if punct.as_char() == ';' => returning = in_return,
            TokenTree::Group(group) => {
                let after_merge = matches!(
                    index.checked_sub(1).and_then(|before| trees.get(before)),
                    Some(TokenTree::Ident(name)) if name == "merge_axum"
                );
                if after_merge && group.delimiter() == Delimiter::Parenthesis {
                    let content = compact(&group.stream());
                    if content.starts_with("crate::pwa::router()") {
                        *mounted = true;
                    } else if content == HEALTH && !returning {
                        ends.push(group.span().byte_range().end);
                    }
                }
                let inner = group.stream().into_iter().collect::<Vec<_>>();
                scan(&inner, returning, ends, mounted);
            }
            _ => {}
        }
    }
}

/// How many `.merge_axum(rullst::health::health_router())` calls `source`
/// has, and whether it mounts the PWA router.
pub(super) fn health_merges(source: &str) -> Result<(usize, bool), Box<dyn Error>> {
    let trees = source
        .parse::<TokenStream>()?
        .into_iter()
        .collect::<Vec<_>>();
    let mut ends = Vec::new();
    let mut mounted = false;
    scan(&trees, false, &mut ends, &mut mounted);
    Ok((ends.len(), mounted))
}

/// `source` with the PWA router merged after the health probes, or `None`
/// when it is already mounted.
pub(super) fn mount_router(source: &str) -> Result<Option<String>, Box<dyn Error>> {
    syn::parse_file(source)?;
    let trees = source
        .parse::<TokenStream>()?
        .into_iter()
        .collect::<Vec<_>>();
    let mut ends = Vec::new();
    let mut mounted = false;
    scan(&trees, false, &mut ends, &mut mounted);
    if mounted {
        return Ok(None);
    }
    let [end] = ends.as_slice() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "make:pwa did not find exactly one `.merge_axum(rullst::health::health_router())` in this file",
        )
        .into());
    };
    let mut updated = source.to_owned();
    if !updated.is_char_boundary(*end) {
        return Err(io::Error::other("invalid router source range").into());
    }
    // The new call is indented like the line that holds the health merge.
    let line = updated
        .get(..*end)
        .and_then(|before| before.rsplit('\n').next())
        .unwrap_or_default();
    let indent = line
        .get(..line.len() - line.trim_start().len())
        .unwrap_or_default();
    let mount = format!(
        "\n{indent}// Service worker and offline page (cargo rullst make:pwa).\n{indent}.merge_axum(crate::pwa::router().into_axum())"
    );
    updated.insert_str(*end, &mount);
    syn::parse_file(&updated)?;
    Ok(Some(updated))
}

/// The `.layer(...)` calls whose argument names `headers_middleware`: where
/// each receiver ends and where its `.layer` starts.
#[derive(Default)]
struct HeaderLayers {
    found: Vec<(usize, usize)>,
}

impl<'ast> Visit<'ast> for HeaderLayers {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let names_headers = |tokens: TokenStream| {
            let mut pending = vec![tokens];
            while let Some(tokens) = pending.pop() {
                for tree in tokens {
                    match tree {
                        TokenTree::Ident(ident) if ident == "headers_middleware" => return true,
                        TokenTree::Group(group) => pending.push(group.stream()),
                        _ => {}
                    }
                }
            }
            false
        };
        if node.method == "layer"
            && node.args.len() == 1
            && node
                .args
                .first()
                .is_some_and(|argument| names_headers(quote::ToTokens::to_token_stream(argument)))
        {
            self.found.push((
                node.receiver.span().byte_range().end,
                node.dot_token.span().byte_range().start,
            ));
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// `source` (the `--router` file) with the PWA router mounted: after its one
/// health merge, else right inside its one `headers_middleware` layer, so
/// the routes stay outside the CSRF layer and get the security headers.
/// Unchanged when the PWA router is already mounted.
pub(super) fn mount_router_in(source: &str) -> Result<String, Box<dyn Error>> {
    let (merges, mounted) = health_merges(source)?;
    if mounted {
        return Ok(source.to_owned());
    }
    match merges {
        0 => {}
        1 => return Ok(mount_router(source)?.unwrap_or_else(|| source.to_owned())),
        count => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "it merges `{HEALTH}` {count} times; add `.merge_axum(crate::pwa::router().into_axum())` by hand after the one the served router uses"
                ),
            )
            .into());
        }
    }
    let mut layers = HeaderLayers::default();
    layers.visit_file(&syn::parse_file(source)?);
    let [(end, dot)] = layers.found.as_slice() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "it has neither a `.merge_axum(rullst::health::health_router())` nor exactly one `.layer(...)` of `rullst::security::headers_middleware` to mount the PWA routes next to; add `.merge_axum(crate::pwa::router().into_axum())` by hand, outside the CSRF layer",
        )
        .into());
    };
    let mut updated = source.to_owned();
    if !updated.is_char_boundary(*end) || !updated.is_char_boundary(*dot) {
        return Err(io::Error::other("invalid router source range").into());
    }
    // The new call is indented like the line of the header layer.
    let line = updated
        .get(..*dot)
        .and_then(|before| before.rsplit('\n').next())
        .unwrap_or_default();
    let indent = line
        .get(..line.len() - line.trim_start().len())
        .unwrap_or_default();
    let mount = format!(
        "\n{indent}// Service worker and offline page (cargo rullst make:pwa).\n{indent}.merge_axum(crate::pwa::router().into_axum())"
    );
    updated.insert_str(*end, &mount);
    syn::parse_file(&updated)?;
    Ok(updated)
}

/// `source` declaring the `pwa` module, or `None` when it already does.
/// A library root makes it public so a binary can reach it.
pub(super) fn declare_module(
    source: &str,
    library: bool,
) -> Result<Option<String>, Box<dyn Error>> {
    let file = syn::parse_file(source)?;
    let declared = file
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Mod(module) if module.ident == "pwa"));
    if declared {
        return Ok(None);
    }
    let visibility = if library { "pub " } else { "" };
    let updated = super::super::insert_module_declaration(
        source,
        "pwa",
        &format!(
            "// Web app manifest, service worker and offline page (cargo rullst make:pwa).\n{visibility}mod pwa;"
        ),
    );
    syn::parse_file(&updated)?;
    Ok(Some(updated))
}
