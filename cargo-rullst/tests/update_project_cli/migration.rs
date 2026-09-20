//! Offline updater protocol evidence with two distinct local package versions.
//! These tiny packages do not establish actual framework API compatibility.
use super::{
    Fixture,
    application::{digest, invoke},
    review::verified,
    text,
};
use std::{fs, process::Command};

fn major_fixture() -> Fixture {
    let fixture = Fixture::new("12.1.0", "12.1.0");
    let next = fixture.app.join("vendor/framework-next");
    fs::create_dir_all(next.join("src")).unwrap();
    fs::write(
        next.join("Cargo.toml"),
        format!(
            "[package]\nname = \"rullst-core\"\nversion = \"{}\"\nedition = \"2024\"\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    fs::write(next.join("src/lib.rs"), "pub fn marker() {}\n").unwrap();
    fs::write(fixture.app.join("Cargo.toml"),
        "[package]\nname = \"prepared-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst-core = \"12.1.0\"\n\n[patch.crates-io]\nprevious = { package = \"rullst-core\", path = \"vendor/framework\" }\nnext = { package = \"rullst-core\", path = \"vendor/framework-next\" }\n"
    ).unwrap();
    fs::write(fixture.app.join("src/main.rs"),
        "// preserved application source\nfn main() { rullst_core::marker(); }\n#[test] fn existing_api() { rullst_core::marker(); }\n"
    ).unwrap();
    fs::write(fixture.app.join("build.rs"),
        "fn main() { std::fs::write(std::env::var(\"RULLST_PROJECT_BUILD_MARKER\").unwrap(), \"executed\").unwrap(); }\n"
    ).unwrap();
    let lock = Command::new("cargo")
        .current_dir(&fixture.app)
        .args(["generate-lockfile", "--offline"])
        .output()
        .unwrap();
    assert!(lock.status.success(), "{}", text(&lock));
    assert_eq!(
        resolved_version(&fs::read(fixture.app.join("Cargo.lock")).unwrap()),
        "12.1.0"
    );
    fixture
}

fn resolved_version(lock: &[u8]) -> String {
    let lock: toml::Value = toml::from_str(std::str::from_utf8(lock).unwrap()).unwrap();
    let packages = lock["package"].as_array().unwrap();
    let framework: Vec<_> = packages
        .iter()
        .filter(|p| p["name"].as_str() == Some("rullst-core"))
        .collect();
    assert_eq!(framework.len(), 1);
    framework[0]["version"].as_str().unwrap().to_owned()
}

#[test]
fn v12_1_to_v13_preparation_verification_application_and_recovery_use_distinct_packages() {
    assert_eq!(
        semver::Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap()
            .major,
        13
    );
    let fixture = major_fixture();
    let manifest = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let lock = fs::read(fixture.app.join("Cargo.lock")).unwrap();
    let source = fs::read(fixture.app.join("src/main.rs")).unwrap();
    let stage = verified(&fixture);
    let candidate_lock = fs::read(stage.join("candidate/Cargo.lock")).unwrap();
    assert_eq!(resolved_version(&candidate_lock), env!("CARGO_PKG_VERSION"));
    assert_ne!(candidate_lock, lock);
    assert_eq!(
        fs::read(stage.join("candidate/src/main.rs")).unwrap(),
        source
    );
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), manifest);
    assert_eq!(fs::read(fixture.app.join("Cargo.lock")).unwrap(), lock);
    assert!(!fixture.app.join("target").exists());
    let approval = digest(&fixture, &stage);

    // The major transition keeps the same stale-source and explicit-review gate.
    fs::write(
        fixture.app.join("src/main.rs"),
        "// concurrent user edit\nfn main() {}\n",
    )
    .unwrap();
    assert!(
        !invoke(&fixture, "apply", &stage, &approval)
            .status
            .success()
    );
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), manifest);
    fs::write(fixture.app.join("src/main.rs"), &source).unwrap();
    assert!(
        !invoke(&fixture, "apply", &stage, &"0".repeat(64))
            .status
            .success()
    );
    fs::remove_file(fixture.base.join("build-executed")).unwrap();
    let applied = invoke(&fixture, "apply", &stage, &approval);
    assert!(applied.status.success(), "{}", text(&applied));
    assert_eq!(
        fs::read(fixture.app.join("Cargo.lock")).unwrap(),
        candidate_lock
    );
    assert_eq!(fs::read(fixture.app.join("src/main.rs")).unwrap(), source);
    assert!(!fixture.base.join("build-executed").exists());
    assert!(
        !fixture
            .app
            .join("src/controllers/age_controller.rs")
            .exists()
    );
    assert!(
        !fixture
            .app
            .join("src/controllers/privacy_controller.rs")
            .exists()
    );

    let recovered = invoke(&fixture, "recover", &stage, &approval);
    assert!(recovered.status.success(), "{}", text(&recovered));
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), manifest);
    assert_eq!(fs::read(fixture.app.join("Cargo.lock")).unwrap(), lock);
    assert_eq!(fs::read(fixture.app.join("src/main.rs")).unwrap(), source);
    assert!(!fixture.base.join("build-executed").exists());
}
