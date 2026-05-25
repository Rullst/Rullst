extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod html_parser;

/// A macro for writing HTML inline in Rust.
/// It compiles down to highly optimized string concatenations at compile time,
/// and automatically escapes dynamic variables to prevent XSS.
/// 
/// # Example
/// ```rust,ignore
/// let name = "Mundo";
/// let page = html! {
///     <div class="container">
///         <h1>"Olá, " {name} "!"</h1>
///     </div>
/// };
/// ```
#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    let node = parse_macro_input!(input as html_parser::HtmlNode);
    let expanded = node.to_tokens();
    expanded.into()
}
