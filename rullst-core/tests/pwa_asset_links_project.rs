//! `AssetLinks::from_project` reads the process environment first. It changes
//! process-wide state, so it runs alone in this test binary.
#![allow(clippy::unwrap_used)]

use rullst_core::pwa::AssetLinks;

#[test]
fn the_process_environment_configures_the_project_links() {
    // The only test in this process, so no other thread reads the variable.
    unsafe {
        std::env::set_var(
            "RULLST_ANDROID_ASSET_LINKS",
            "com.acme.notes=14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5",
        );
    }
    let links = AssetLinks::from_project().unwrap();
    assert_eq!(links.apps().len(), 1);
    assert_eq!(links.apps()[0].package_name(), "com.acme.notes");
    assert_eq!(
        links.apps()[0].sha256_cert_fingerprints(),
        [
            "14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5"
        ]
    );
}
