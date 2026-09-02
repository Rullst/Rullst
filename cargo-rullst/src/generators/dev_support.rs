use crate::ui::dash_tui::LogMsg;
use colored::Colorize;
use notify::EventKind;
use quote::quote;
use std::path::Path;
use syn::{Macro, visit_mut::VisitMut};
use tokio::sync::mpsc;

const MAX_BUILD_DIAGNOSTIC_CHARS: usize = 16 * 1024;

pub(super) const fn is_actionable_event(kind: EventKind) -> bool {
    !matches!(kind, EventKind::Access(_) | EventKind::Other)
}

struct HtmlStripper;

impl VisitMut for HtmlStripper {
    fn visit_macro_mut(&mut self, macro_node: &mut Macro) {
        if macro_node.path.is_ident("html") {
            macro_node.tokens = proc_macro2::TokenStream::new();
        }
        syn::visit_mut::visit_macro_mut(self, macro_node);
    }
}

pub(super) fn did_logic_change(old_source: &str, new_source: &str) -> bool {
    let mut old_ast = match syn::parse_file(old_source) {
        Ok(ast) => ast,
        Err(_) => return true,
    };
    let mut new_ast = match syn::parse_file(new_source) {
        Ok(ast) => ast,
        Err(_) => return true,
    };
    let mut stripper = HtmlStripper;
    stripper.visit_file_mut(&mut old_ast);
    stripper.visit_file_mut(&mut new_ast);
    quote!(#old_ast).to_string() != quote!(#new_ast).to_string()
}

pub(super) fn hot_reload_profile_available(root: &Path) -> bool {
    let Ok(manifest) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return false;
    };
    let Ok(manifest) = toml::from_str::<toml::Value>(&manifest) else {
        return false;
    };
    let has_cdylib = manifest
        .get("lib")
        .and_then(|lib| lib.get("crate-type"))
        .and_then(toml::Value::as_array)
        .is_some_and(|crate_types| {
            crate_types
                .iter()
                .any(|crate_type| crate_type.as_str() == Some("cdylib"))
        });
    let has_initializer = std::fs::read_to_string(root.join("src/lib.rs"))
        .ok()
        .is_some_and(|source| source.contains("rullst_router_init"));
    has_cdylib && has_initializer
}

pub(super) fn generate_reload_token() -> String {
    rand::random::<[u8; 32]>()
        .into_iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn bounded_diagnostic(bytes: &[u8]) -> String {
    let diagnostic = String::from_utf8_lossy(bytes);
    diagnostic
        .chars()
        .take(MAX_BUILD_DIAGNOSTIC_CHARS)
        .collect()
}

pub(super) async fn report_watcher_message(
    sender: &mpsc::Sender<LogMsg>,
    dashboard: bool,
    message: String,
    is_error: bool,
) {
    let rendered = if is_error {
        message.red().to_string()
    } else {
        message.cyan().to_string()
    };
    if dashboard {
        let _ = sender.send(LogMsg::System(rendered)).await;
    } else if is_error {
        eprintln!("{rendered}");
    } else {
        println!("{rendered}");
    }
}

#[cfg(test)]
mod tests {
    use notify::EventKind;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};
    use std::fs;

    #[test]
    fn watcher_ignores_reads_and_meta_events_but_accepts_changes() {
        assert!(!super::is_actionable_event(EventKind::Access(
            AccessKind::Any
        )));
        assert!(!super::is_actionable_event(EventKind::Other));
        assert!(super::is_actionable_event(EventKind::Create(
            CreateKind::Any
        )));
        assert!(super::is_actionable_event(EventKind::Modify(
            ModifyKind::Any
        )));
        assert!(super::is_actionable_event(EventKind::Remove(
            RemoveKind::Any
        )));
        assert!(super::is_actionable_event(EventKind::Any));
    }

    #[test]
    fn build_diagnostics_are_bounded_and_lossy_utf8_safe() {
        let oversized = vec![b'x'; super::MAX_BUILD_DIAGNOSTIC_CHARS + 50];
        assert_eq!(
            super::bounded_diagnostic(&oversized).len(),
            super::MAX_BUILD_DIAGNOSTIC_CHARS
        );
        assert!(!super::bounded_diagnostic(&[0xff]).is_empty());
    }

    #[test]
    fn hot_reload_profile_requires_cdylib_and_explicit_initializer() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("src")).expect("source directory");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n[lib]\ncrate-type=['cdylib','rlib']\n",
        )
        .expect("manifest");
        fs::write(
            temp.path().join("src/lib.rs"),
            "pub extern \"C\" fn rullst_router_init() {}",
        )
        .expect("library");
        assert!(super::hot_reload_profile_available(temp.path()));

        fs::write(temp.path().join("src/lib.rs"), "pub fn router() {}").expect("library");
        assert!(!super::hot_reload_profile_available(temp.path()));
    }

    #[test]
    fn ast_classifier_distinguishes_view_tokens_from_rust_logic() {
        let before = "fn page() { html! { <p>before</p> } } fn value() -> u8 { 1 }";
        let view_only = "fn page() { html! { <p>after</p> } } fn value() -> u8 { 1 }";
        let logic = "fn page() { html! { <p>before</p> } } fn value() -> u8 { 2 }";
        assert!(!super::did_logic_change(before, view_only));
        assert!(super::did_logic_change(before, logic));
        assert!(super::did_logic_change(before, "not valid Rust"));
    }

    #[test]
    fn reload_token_has_256_random_bits_encoded_as_hex() {
        let token = super::generate_reload_token();
        assert_eq!(token.len(), 64);
        assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}
