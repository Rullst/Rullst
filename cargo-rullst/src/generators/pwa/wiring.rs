//! Mounts `crate::pwa::router()` next to the health probes and declares the
//! `pwa` module in the crate root.
//!
//! Every HTML starter merges `rullst::health::health_router()` into its
//! router exactly once; the PWA routes are merged right after it, outside the
//! CSRF layer (they answer GET only) and inside any later header layer. A
//! router without that merge is not recognized and nothing is written.
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::{error::Error, io};

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
            "make:pwa did not find exactly one `.merge_axum(rullst::health::health_router())` in the crate root; add `.merge_axum(crate::pwa::router().into_axum())` to your router by hand (see PWA guide)",
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
    let mut updated = source.to_owned();
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&format!(
        "\n// Web app manifest, service worker and offline page (cargo rullst make:pwa).\n{visibility}mod pwa;\n"
    ));
    syn::parse_file(&updated)?;
    Ok(Some(updated))
}
