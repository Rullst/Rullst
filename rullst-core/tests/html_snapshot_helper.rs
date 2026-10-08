//! The HTML snapshot assertion from an integration test: snapshots resolve
//! against this crate's `tests/snapshots` folder.

use rullst_core::testing::{SnapshotOptions, assert_html_snapshot};

#[test]
fn integration_tests_share_the_crate_snapshot_folder() {
    let nonce = "per-response-nonce";
    let page = format!(
        "<section class=\"hero\"><h1>Snapshot</h1><form><input type=\"hidden\" \
         name=\"_token\" value=\"{nonce}\"></form></section>"
    );
    assert_html_snapshot!(
        "testing_helper_example",
        page,
        SnapshotOptions::new().mask_csrf_token()
    );
}
