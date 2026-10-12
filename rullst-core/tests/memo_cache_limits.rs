//! Limits of the process-wide memoize cache. The cache is global, so these
//! checks run in their own test binary.

use rullst_core::cache::memory;

#[test]
fn entries_up_to_256_kib_are_cached_and_several_fit_in_the_store() {
    // Key plus value of 262,144 bytes is the largest cached entry.
    let largest = "x".repeat(262_144 - "entry-0".len());
    for index in 0..5 {
        memory::set(&format!("entry-{index}"), &largest);
    }
    for index in 0..5 {
        assert!(
            memory::get(&format!("entry-{index}")).is_some(),
            "entry-{index} was not cached"
        );
    }

    let over = "x".repeat(262_145 - "over".len());
    memory::set("over", &over);
    assert_eq!(memory::get("over"), None);
}
