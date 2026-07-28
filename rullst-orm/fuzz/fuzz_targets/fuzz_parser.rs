#![no_main]
use libfuzzer_sys::fuzz_target;

// Include the parser logic directly to test it without needing to restructure
// the `rullst-orm-macros` proc-macro crate.
mod internal_parser {
    include!("../../rullst-orm-macros/src/parser.rs");
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // syn::parse2 will stack overflow on deeply nested structures.
        // We restrict the number of nesting tokens and recursive operators to avoid this.
        let nesting = s.chars().filter(|c| "<({[|&*-!=+".contains(*c)).count();
        let keywords = s.matches("return").count()
            + s.matches("yield").count()
            + s.matches("await").count()
            + s.matches("break").count()
            + s.matches("continue").count();

        if nesting + keywords > 64 {
            return;
        }

        // We attempt to parse the random string as a Rust TokenStream
        if let Ok(ts) = s.parse::<proc_macro2::TokenStream>() {

            // Attempt to parse it as a struct definition (DeriveInput)
            if let Ok(ast) = syn::parse2::<syn::DeriveInput>(ts) {
                // Fuzz our parser! It should never panic, only return Ok or Err.
                let _ = internal_parser::parse(&ast);
            }
        }
    }
});
