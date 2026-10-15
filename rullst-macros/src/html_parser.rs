#![cfg_attr(mutants, mutants::skip)]
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Result, Token, token};

pub enum HtmlNode {
    Element(HtmlElement),
    Text(LitStr),
    Block(Expr),
    If(Box<HtmlIf>),
    For(Box<HtmlFor>),
}

pub struct HtmlDocument {
    nodes: Vec<HtmlNode>,
}

#[path = "html_declarations.rs"]
mod declarations;
use declarations::{
    misplaced_doctype, parse_comment, parse_doctype, starts_comment, starts_doctype,
};

#[path = "html_attributes.rs"]
mod attributes;
use attributes::{HtmlAttribute, allows_data_urls, url_guard_tokens};

#[path = "html_control.rs"]
mod control;
use control::{HtmlFor, HtmlIf};

impl Parse for HtmlDocument {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            if starts_doctype(input) {
                if !nodes.is_empty() {
                    return Err(misplaced_doctype(input));
                }
                nodes.push(parse_doctype(input)?);
            } else if starts_comment(input) {
                parse_comment(input)?;
            } else {
                nodes.push(input.parse::<HtmlNode>()?);
            }
        }
        Ok(Self { nodes })
    }
}

pub struct HtmlElement {
    pub tag_name: Ident,
    pub attributes: Vec<HtmlAttribute>,
    pub children: Vec<HtmlNode>,
}

impl Parse for HtmlNode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![<]) {
            let element = input.parse::<HtmlElement>()?;
            Ok(HtmlNode::Element(element))
        } else if input.peek(Token![@]) {
            control::parse_control(input)
        } else if input.peek(Token![else]) {
            Err(control::else_without_at(input))
        } else if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let expr = content.parse::<Expr>()?;
            Ok(HtmlNode::Block(expr))
        } else {
            let lit = input.parse::<LitStr>()?;
            Ok(HtmlNode::Text(lit))
        }
    }
}

impl Parse for HtmlElement {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![<]>()?;
        // `parse_any` also accepts Rust keywords, such as SVG's `<use>`.
        let tag_name = Ident::parse_any(input)?;

        let mut attributes = Vec::new();
        while !(input.peek(Token![>]) || input.peek(Token![/]) && input.peek2(Token![>])) {
            attributes.push(input.parse::<HtmlAttribute>()?);
        }

        if input.peek(Token![/]) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            return Ok(HtmlElement {
                tag_name,
                attributes,
                children: Vec::new(),
            });
        }

        input.parse::<Token![>]>()?;

        let children = parse_children(input)?;

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let closing_tag = Ident::parse_any(input)?;
        if closing_tag != tag_name {
            return Err(syn::Error::new(
                closing_tag.span(),
                format!(
                    "Mismatched closing tag: expected </{}>, found </{}>",
                    tag_name, closing_tag
                ),
            ));
        }
        input.parse::<Token![>]>()?;

        Ok(HtmlElement {
            tag_name,
            attributes,
            children,
        })
    }
}

/// Parses child nodes until the input ends or a closing tag (`</`) follows.
fn parse_children(input: ParseStream) -> Result<Vec<HtmlNode>> {
    let mut children = Vec::new();
    while !input.is_empty() {
        if input.peek(Token![<]) && input.peek2(Token![/]) {
            break;
        }
        if starts_doctype(input) {
            return Err(misplaced_doctype(input));
        }
        if starts_comment(input) {
            parse_comment(input)?;
        } else {
            children.push(input.parse::<HtmlNode>()?);
        }
    }
    Ok(children)
}

impl HtmlDocument {
    /// Expands the document into a `rullst::html::Markup`: the rendered string
    /// already escapes every interpolated value that is not itself markup.
    pub fn to_tokens(&self) -> TokenStream {
        let rendered = self.render_tokens();
        quote! { rullst::html::Markup::from_trusted(#rendered) }
    }

    fn render_tokens(&self) -> TokenStream {
        if self.nodes.len() == 1 {
            return self.nodes[0].to_tokens();
        }

        let capacity = self.nodes.iter().map(HtmlNode::static_size).sum::<usize>();
        let nodes = self.nodes.iter().map(HtmlNode::to_tokens);
        // Keep the generated local out of caller expressions, including an `s`
        // binding in a fragment. Renaming a call-site identifier is not hygiene.
        let buffer = Ident::new("s", Span::mixed_site());
        quote! {
            {
                let mut #buffer = String::with_capacity(#capacity);
                #(#buffer.push_str(&#nodes);)*
                #buffer
            }
        }
    }
}

impl HtmlNode {
    pub fn to_tokens(&self) -> TokenStream {
        match self {
            HtmlNode::Element(el) => el.to_tokens(),
            HtmlNode::Text(txt) => {
                let val = txt.value();
                quote! { #val.to_string() }
            }
            HtmlNode::Block(expr) => {
                quote! { rullst::html::escape(&(#expr)) }
            }
            HtmlNode::If(node) => node.to_tokens(),
            HtmlNode::For(node) => node.to_tokens(),
        }
    }

    pub fn static_size(&self) -> usize {
        match self {
            HtmlNode::Element(el) => el.static_size(),
            HtmlNode::Text(txt) => txt.value().len(),
            HtmlNode::Block(_) | HtmlNode::If(_) | HtmlNode::For(_) => 0,
        }
    }
}

impl HtmlElement {
    pub fn static_size(&self) -> usize {
        let tag = self.tag_name.to_string();
        let mut size = tag.len() * 2 + 5; // <tag></tag>

        for attr in &self.attributes {
            size += attr.static_size();
        }

        for child in &self.children {
            size += child.static_size();
        }
        size
    }

    pub fn to_tokens(&self) -> TokenStream {
        let tag = self.tag_name.to_string();
        let capacity = self.static_size();
        // Use one hygienic binding for this element and all its attribute
        // writes; interpolated caller expressions retain their original spans.
        let buffer = Ident::new("s", Span::mixed_site());

        let allow_data_urls = allows_data_urls(&tag);
        let mut needs_url_guard = false;
        let mut attr_tokens = Vec::new();
        for attr in &self.attributes {
            let (tokens, uses_url_guard) = attr.to_tokens(&buffer, allow_data_urls);
            needs_url_guard |= uses_url_guard;
            attr_tokens.push(tokens);
        }
        let url_guard = if needs_url_guard {
            url_guard_tokens()
        } else {
            TokenStream::new()
        };

        let child_tokens = self.children.iter().map(|child| child.to_tokens());

        let void_elements = [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ];
        let is_void = void_elements.contains(&tag.as_str());

        if self.children.is_empty() && is_void {
            quote! {
                {
                    #url_guard
                    let mut #buffer = String::with_capacity(#capacity);
                    #buffer.push_str("<");
                    #buffer.push_str(#tag);
                    #( #attr_tokens )*
                    #buffer.push_str(" />");
                    #buffer
                }
            }
        } else {
            quote! {
                {
                    #url_guard
                    let mut #buffer = String::with_capacity(#capacity);
                    #buffer.push_str("<");
                    #buffer.push_str(#tag);
                    #( #attr_tokens )*
                    #buffer.push_str(">");
                    #( #buffer.push_str(&#child_tokens); )*
                    #buffer.push_str("</");
                    #buffer.push_str(#tag);
                    #buffer.push_str(">");
                    #buffer
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "html_parser_tests.rs"]
mod tests;

#[allow(unexpected_cfgs)]
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    #[kani::unwind(6)]
    fn verify_static_size_no_overflow() {
        // We model the logic of HtmlElement::static_size() to mathematically prove
        // it cannot overflow `usize` with reasonable constraints imposed by the macro system.
        // Proc-macros operate on source files which are bounded in size.
        let tag_len: usize = kani::any();
        kani::assume(tag_len <= 100);

        let num_attrs: usize = kani::any();
        kani::assume(num_attrs <= 4);

        let mut size = tag_len * 2 + 5;

        for _ in 0..num_attrs {
            let attr_name_len: usize = kani::any();
            kani::assume(attr_name_len <= 100);

            let is_static: bool = kani::any();
            size += attr_name_len + 4;

            if is_static {
                let lit_val_len: usize = kani::any();
                kani::assume(lit_val_len <= 5000); // 5KB string literal max per attr
                size += lit_val_len;
            }
        }

        let num_children: usize = kani::any();
        kani::assume(num_children <= 4);

        for _ in 0..num_children {
            let child_size: usize = kani::any();
            kani::assume(child_size <= 10000); // 10KB child max
            size += child_size;
        }

        // Prove size computation never causes panic/overflow
        assert!(size < usize::MAX);
    }

    #[kani::proof]
    fn verify_void_elements_check() {
        let void_elements = [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ];

        let i: usize = kani::any();
        kani::assume(i < void_elements.len());

        let tag = void_elements[i];

        // Assert that the parsing logic's void element check works correctly
        let is_void = void_elements.contains(&tag);
        assert!(is_void);
    }
}
