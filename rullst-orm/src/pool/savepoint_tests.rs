use super::{HANDLE_GATES, SharedTransaction, handle_gate};
use std::sync::Arc;

fn handle() -> SharedTransaction {
    Arc::new(tokio::sync::Mutex::new(None))
}

fn key(transaction: &SharedTransaction) -> usize {
    Arc::as_ptr(transaction).addr()
}

#[test]
fn a_handle_keeps_one_gate_while_it_is_in_use() {
    let first = handle();
    let gate = handle_gate(&first);
    // Registering another handle prunes the registry, which must keep every
    // gate that is still held.
    let second = handle();
    let _other = handle_gate(&second);
    assert!(Arc::ptr_eq(&gate, &handle_gate(&first)));
}

#[test]
fn released_gates_leave_the_registry() {
    let finished = handle();
    drop(handle_gate(&finished));
    let finished_key = key(&finished);
    // `finished` stays alive, so its address cannot be reused meanwhile.
    let next = handle();
    let _gate = handle_gate(&next);
    let gates = HANDLE_GATES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(!gates.contains_key(&finished_key));
    assert!(gates.contains_key(&key(&next)));
}
