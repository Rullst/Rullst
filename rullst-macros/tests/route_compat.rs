#![allow(deprecated)]

#[rullst_macros::route]
async fn preserved_handler(value: u32) -> Result<u32, &'static str> {
    Ok(value.saturating_add(1))
}

#[test]
fn legacy_route_marker_preserves_async_signature_and_arguments() {
    let future = preserved_handler(41);
    drop(future);
}
