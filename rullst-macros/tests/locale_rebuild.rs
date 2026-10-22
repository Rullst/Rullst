//! The rebuild trigger documented for `catalog!`: with a `build.rs` that
//! prints `cargo::rerun-if-changed=locales`, adding or removing a locale
//! file rebuilds the crate, and editing one rebuilds it through the
//! `include_bytes!` that `catalog!` emits. Without a build script, Cargo
//! does not notice a new file. Small probe crates are built with the real
//! Cargo before and after each change.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BUILD_RS: &str = r#"fn main() {
    // Rebuild when a locale file is added or removed; `catalog!` already
    // tracks edits to the files it read.
    println!("cargo::rerun-if-changed=locales");
}
"#;

/// Stand-ins for the `rullst::i18n` runtime: the probe prints the tags
/// that `catalog!` found.
const MAIN_RS: &str = r#"extern crate self as rullst;

pub mod i18n {
    pub struct Locales(pub &'static [&'static str]);

    impl Locales {
        pub const fn __from_catalog(tags: &'static [&'static str], _: usize) -> Self {
            Locales(tags)
        }
    }

    pub use rullst_macros::__i18n_translate as __translate;
}

mod catalog {
    rullst_macros::i18n_catalog!(default = "en");
}

fn main() {
    println!("{}", catalog::LOCALES.0.join(","));
}
"#;

struct Probe {
    root: PathBuf,
    target: PathBuf,
}

impl Probe {
    /// A probe package named `name`, with the documented `build.rs` or none.
    fn new(name: &str, build_script: bool) -> Self {
        let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
        let root = tmp.join(format!("rullst-{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("locales")).unwrap();
        let macros = Path::new(env!("CARGO_MANIFEST_DIR"));
        // A TOML literal string keeps Windows backslashes as they are.
        let manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n\
             edition = \"2024\"\npublish = false\n\n[dependencies]\n\
             rullst-macros = {{ path = '{}' }}\n\n[workspace]\n",
            macros.display()
        );
        fs::write(root.join("Cargo.toml"), manifest).unwrap();
        // The workspace lockfile pins the versions already downloaded, so
        // the probe resolves offline.
        fs::copy(macros.join("../Cargo.lock"), root.join("Cargo.lock")).unwrap();
        if build_script {
            fs::write(root.join("build.rs"), BUILD_RS).unwrap();
        }
        fs::write(root.join("src/main.rs"), MAIN_RS).unwrap();
        let probe = Self {
            root,
            target: tmp.join("rullst-locale-rebuild-target"),
        };
        probe.locale("en", "greeting = \"Hello\"\n");
        probe
    }

    fn locale(&self, tag: &str, text: &str) {
        fs::write(self.root.join("locales").join(format!("{tag}.toml")), text).unwrap();
    }

    fn remove(&self, tag: &str) {
        fs::remove_file(self.root.join("locales").join(format!("{tag}.toml"))).unwrap();
    }

    /// Builds and runs the probe: its output, or the build's errors.
    fn run(&self) -> Result<String, String> {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let output = Command::new(cargo)
            .args(["run", "--quiet", "--offline"])
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", &self.target)
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .output()
            .unwrap();
        if output.status.success() {
            Ok(String::from_utf8(output.stdout).unwrap().trim().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn adding_editing_and_removing_locale_files_rebuilds_the_crate() {
    // Control: without a build script the new file is not noticed.
    let control = Probe::new("locale-rebuild-control", false);
    assert_eq!(control.run().as_deref(), Ok("en"));
    control.locale("pt-BR", "greeting = \"Olá\"\n");
    assert_eq!(control.run().as_deref(), Ok("en"));

    let probe = Probe::new("locale-rebuild-probe", true);
    assert_eq!(probe.run().as_deref(), Ok("en"));

    probe.locale("pt-BR", "greeting = \"Olá\"\n");
    assert_eq!(probe.run().as_deref(), Ok("en,pt-BR"));

    // An edit that drops a key fails the build: the edit was seen.
    probe.locale("pt-BR", "saudacao = \"Olá\"\n");
    let error = probe.run().unwrap_err();
    assert!(
        error.contains("locales/pt-BR.toml: missing key `greeting`"),
        "{error}"
    );

    probe.locale("pt-BR", "greeting = \"Oi\"\n");
    probe.locale("fr", "greeting = \"Bonjour\"\n");
    assert_eq!(probe.run().as_deref(), Ok("en,fr,pt-BR"));

    probe.remove("pt-BR");
    assert_eq!(probe.run().as_deref(), Ok("en,fr"));
}
