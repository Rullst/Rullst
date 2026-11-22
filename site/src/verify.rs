//! Checks of each rendered page before anything is written: its canonical URL, its
//! Content-Security-Policy, and that every `/assets/` reference is published and current.

use crate::assets::Assets;

const SITE_URL: &str = "https://rullst.win/";

/// The canonical URL of each published page.
fn canonical(page: &str) -> Option<String> {
    let path = match page {
        "index.html" => "",
        "pt-br/index.html" => "pt-br/",
        "compare/index.html" => "compare/",
        "pt-br/comparar/index.html" => "pt-br/comparar/",
        "privacy.html" | "pt-br/privacy.html" => page,
        _ => return None,
    };
    Some(format!("{SITE_URL}{path}"))
}

fn is_landing(page: &str) -> bool {
    matches!(page, "index.html" | "pt-br/index.html")
}

/// The `/assets/...` paths an `href` or `src` attribute references, with their query.
fn asset_references(html: &str) -> Vec<&str> {
    let mut found = Vec::new();
    for attribute in ["href=\"/assets/", "src=\"/assets/"] {
        let mut rest = html;
        while let Some(start) = rest.find(attribute) {
            let value = &rest[start + attribute.len() - "assets/".len()..];
            let end = value.find('"').unwrap_or(value.len());
            found.push(&value[..end]);
            rest = &value[end..];
        }
    }
    found
}

/// One page's canonical URL, CSP, asset references and benchmark containers.
pub fn page(page: &str, html: &str, assets: &Assets) -> Result<(), String> {
    let url = canonical(page).ok_or(format!("{page} is not a known page"))?;
    if !html.contains(&format!("<link rel=\"canonical\" href=\"{url}\" />")) {
        return Err(format!("{page} must declare the canonical URL {url}"));
    }
    if !html.contains("<meta http-equiv=\"Content-Security-Policy\" content=\"") {
        return Err(format!("{page} must declare a Content-Security-Policy"));
    }
    let references = asset_references(html);
    for reference in &references {
        let (path, query) = reference.split_once('?').unwrap_or((reference, ""));
        if !Assets::publishes(path) {
            return Err(format!(
                "{page} references /{path}, which the root site does not publish"
            ));
        }
        let current = match path {
            "assets/site.css" => Some(assets.css.as_str()),
            "assets/site.js" => Some(assets.js.as_str()),
            _ => None,
        };
        if let Some(current) = current
            && format!("/{reference}") != current
        {
            return Err(format!(
                "{page} references /{path}?{query}; the current one is {current}"
            ));
        }
    }
    let loads = |current: &str| {
        references
            .iter()
            .any(|reference| format!("/{reference}") == current)
    };
    if !loads(&assets.css) || (is_landing(page) && !loads(&assets.js)) {
        return Err(format!(
            "{page} must load the current content-versioned stylesheet and script"
        ));
    }
    let containers: &[&str] = if is_landing(page) {
        &["data-landing-proof", "data-landing-benchmarks"]
    } else if page.contains("compar") {
        &["data-benchmark-results"]
    } else {
        &[]
    };
    for container in containers {
        if html.matches(&format!(" {container}>")).count() != 1 {
            return Err(format!("{page} needs one {container} block"));
        }
    }
    Ok(())
}
