//! `PinnedRunner` was `LinuxExperimental` before 13.0.0-rc.1. The profile is
//! serialized into every stored profile, store binding, job and receipt
//! binding, so its wire name and digest must stay byte-identical. The golden
//! values below were computed with the code before the rename.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// A pinned-runner profile with fixed tool and key digests.
pub(crate) fn pinned_profile() -> ExecutionProfile {
    ExecutionProfile::PinnedRunner {
        tools: ToolIdentity {
            runner: ContentHash::of(b"runner"),
            compiler: ContentHash::of(b"compiler"),
            wasm_toolchain: ContentHash::of(b"wasm_toolchain"),
            runtime: ContentHash::of(b"runtime"),
            launcher: ContentHash::of(b"launcher"),
            syscall_policy: ContentHash::of(b"syscall_policy"),
            filesystem_policy: ContentHash::of(b"filesystem_policy"),
        },
        receipt_key: ContentHash::of(b"receipt_key"),
    }
}

/// `serde_json` of [`pinned_profile`], as 13.0.0-alpha.6 wrote it.
pub(crate) const PINNED_PROFILE_JSON: &str = concat!(
    r#"{"LinuxExperimental":{"tools":{"#,
    r#""runner":"527aa9f431539da8e151d5434d1d5e611d973f601d8e970790882624554146b0","#,
    r#""compiler":"e996bb0ea465fae70d3e3c66b3b6e02d33d2f1eb76d5958720578b6cf359cc2e","#,
    r#""wasm_toolchain":"c035234196b9fb5c48022b4fb0557fdb097fd48f7a5a0fd514c600acb77a807a","#,
    r#""runtime":"d92c6a81b2ff50096bcda80885427d1f59a25b5f483f7055523504925d16ab23","#,
    r#""launcher":"ec9a6e9fe278eb1a471fbab6f40367d8548078b651d9c71581c57c2a6ca379e0","#,
    r#""syscall_policy":"d1fc0456e041f0e4b3cebfa5677c695218cc0f47b4ed23f5e14bb05e098b823f","#,
    r#""filesystem_policy":"a733ecd620d8353b4ef1fe5da42e1a99ed9cd37e040cfe3efc0c28094b30daf0"},"#,
    r#""receipt_key":"55dc6538087457e18e3d4be06f51d41aea63f80163724eae2336bc7b16582978"}}"#,
);

#[test]
fn pinned_runner_writes_and_reads_the_linux_experimental_wire_name() {
    let profile = pinned_profile();
    assert_eq!(
        serde_json::to_string(&profile).unwrap(),
        PINNED_PROFILE_JSON
    );
    assert_eq!(
        serde_json::from_str::<ExecutionProfile>(PINNED_PROFILE_JSON).unwrap(),
        profile
    );
    assert_eq!(
        serde_json::to_string(&ExecutionProfile::Simulation).unwrap(),
        r#""Simulation""#
    );
    // One wire name only: the Rust name is never read or written.
    let renamed = PINNED_PROFILE_JSON.replace("LinuxExperimental", "PinnedRunner");
    assert!(serde_json::from_str::<ExecutionProfile>(&renamed).is_err());
}

#[test]
fn profile_digests_are_unchanged_by_the_rename() {
    assert_eq!(
        pinned_profile().digest().unwrap().as_str(),
        "e459e41c324472237b85405835499a90059bb712abc5a64b8c33eb690bf4c81b"
    );
    assert_eq!(
        ExecutionProfile::Simulation.digest().unwrap().as_str(),
        "4d41c9a3d3313e86f316e6fca9d85e2e41636e5fb4f978710c4fdabc03dd3a83"
    );
}
