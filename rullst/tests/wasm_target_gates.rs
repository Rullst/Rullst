//! The facade must compile for `wasm32` with any feature set, including the
//! defaults. Optional crates declared only for native targets activate no
//! dependency on `wasm32`, so their re-exports must carry the same target guard.

use std::collections::BTreeSet;

const WASM_GUARD: &str = "not(target_arch = \"wasm32\")";

fn native_only_optional_crates() -> BTreeSet<String> {
    let manifest: toml::Table = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    let dependencies = manifest["target"]["cfg(not(target_arch = \"wasm32\"))"]["dependencies"]
        .as_table()
        .unwrap();
    dependencies
        .iter()
        .filter(|(_, spec)| {
            spec.get("optional")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false)
        })
        .map(|(name, _)| name.replace('-', "_"))
        .collect()
}

/// Returns the `#[cfg(...)]` attribute text directly above `line_index`.
fn preceding_cfg(lines: &[&str], line_index: usize) -> String {
    let mut attribute = Vec::new();
    for line in lines[..line_index].iter().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("///") {
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("pub ") || trimmed == "}" {
            break;
        }
        attribute.push(trimmed);
    }
    attribute.reverse();
    attribute.join(" ")
}

#[test]
fn native_only_reexports_are_guarded_for_wasm32() {
    let crates = native_only_optional_crates();
    for expected in ["rullst_orm", "rullst_auth", "rullst_mail", "rullst_studio"] {
        assert!(
            crates.contains(expected),
            "{expected} must stay native-only"
        );
    }

    let source = include_str!("../src/lib.rs");
    let lines = source.lines().collect::<Vec<_>>();
    let mut checked = 0;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let reexported = trimmed
            .strip_prefix("pub use ")
            .and_then(|rest| rest.split([' ', ':', ';']).next())
            .is_some_and(|name| crates.contains(name));
        if reexported || trimmed == "pub mod account_mail;" {
            let cfg = preceding_cfg(&lines, index);
            assert!(
                cfg.contains(WASM_GUARD),
                "`{trimmed}` (line {}) needs a wasm32 guard, found `{cfg}`",
                index + 1
            );
            checked += 1;
        }
    }
    // db::*, orm, rullst_orm, auth, account_mail, mail, messaging, privacy,
    // ai, nexus, capital, studio, security::runtime and security_runtime.
    assert!(checked >= 14, "only {checked} native-only re-exports found");
}
