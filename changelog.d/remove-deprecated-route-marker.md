### Removed

- **Breaking:** the `#[route]` marker attribute of `rullst-macros`
  (`rullst::route`, `rullst_core::route`), deprecated in 12.0.0, was removed.
  It never registered a route: it returned the function unchanged and refused
  arguments. Delete the attribute and register the handler with
  `rullst::routes!` (`get("/path" => handler)`); there is no other
  replacement. `cargo rullst upgrade` reports the attribute and its imports
  (`V13-ROUTE-MARKER-REMOVED`).
