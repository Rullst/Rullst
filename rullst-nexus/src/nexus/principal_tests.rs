//! The administrator identity Nexus policies insert into requests.

use super::*;
use rullst_auth::HasRole;

#[test]
fn the_principal_keeps_its_actor_and_only_the_admin_role() {
    let principal = NexusPrincipal::authenticated("ops-admin");
    assert_eq!(principal.actor_id(), "ops-admin");
    assert_eq!(
        format!("{principal:?}"),
        "NexusPrincipal { actor_id_bytes: 9 }"
    );
    assert!(principal.has_role("NexusAdmin"));
    assert!(!principal.has_role("Admin"));
    assert!(!principal.has_role("nexusadmin"));
}
