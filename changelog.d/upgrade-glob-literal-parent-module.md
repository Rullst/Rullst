### Fixed

- `cargo rullst upgrade` reported a literal of a non-exhaustive Rullst struct
  in a test module that reaches it through `use super::*;`, such as
  `HttpResponse { status, body }` in a `#[path = "x_tests.rs"] mod tests;`
  file, only for review (`V13-NON-EXHAUSTIVE-LITERAL-GLOB`), because each file
  was read alone. The rules now follow the package's `mod` declarations to the
  parent module (and further `use super::*;` links): when it imports the name
  from the struct's Rullst crate the literal is `V13-NON-EXHAUSTIVE-LITERAL`,
  which must change, and when it defines the name or imports it from another
  crate, the application's own type is no longer reported. Literals whose
  origin the module tree cannot tell are still reported for review.
