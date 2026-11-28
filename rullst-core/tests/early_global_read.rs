//! A `RullstConfig::global()` read before `Server` starts no longer fixes the
//! global configuration to its defaults (#858): `Server` replaces them with
//! the project's `Rullst.toml`, so the reads after start-up see the file. The
//! global configuration and the working directory are process-wide, so this
//! binary holds a single test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::{RullstConfig, Server, routes};

#[tokio::test]
async fn server_replaces_the_defaults_an_early_read_installed() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("Rullst.toml"),
        "[storage]\nroot = \"early-read-storage\"\n",
    )
    .unwrap();
    std::env::set_current_dir(project.path()).unwrap();

    // An early read, as code that the router calls while it is built may do.
    let early = RullstConfig::global();
    assert_eq!(early.storage.root, None);

    let server = Server::new(routes![get("/" => || async { "ok" })]);
    // Bind an ephemeral port and stop at once.
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        server.run_with_shutdown(0, async {}),
    )
    .await
    .expect("start-up finishes within the deadline")
    .expect("the server starts and stops");

    assert_eq!(
        RullstConfig::global().storage.root.as_deref(),
        Some("early-read-storage")
    );
    // A reference taken before start-up keeps the defaults.
    assert_eq!(early.storage.root, None);
}
