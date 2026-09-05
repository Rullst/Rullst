#![no_main]
use libfuzzer_sys::fuzz_target;

// Compile the parser as a real module so its `attributes` submodule resolves
// relative to the production source instead of the fuzz target directory.
#[allow(dead_code)]
#[path = "../../../rullst-orm-macros/src/parser.rs"]
mod parser;

fuzz_target!(|data: &[u8]| {
    if data.len() > 2048 {
        return;
    }
    if let Ok(s) = std::str::from_utf8(data) {
        // syn::parse2 will stack overflow on deeply nested structures.
        // We restrict the number of nesting tokens and recursive operators to avoid this.
        let nesting = s.chars().filter(|c| "<({[|&*-!=+".contains(*c)).count();
        let keywords = s.matches("return").count()
            + s.matches("yield").count()
            + s.matches("await").count()
            + s.matches("break").count()
            + s.matches("continue").count();

        if nesting + keywords > 16 {
            return;
        }

        // We attempt to parse the random string as a Rust TokenStream
        if let Ok(ts) = s.parse::<proc_macro2::TokenStream>() {
            // Attempt to parse it as a struct definition (DeriveInput)
            if let Ok(ast) = syn::parse2::<syn::DeriveInput>(ts) {
                // Fuzz our parser! It should never panic, only return Ok or Err.
                let _ = parser::parse(&ast);
            }
        }
    }
});
