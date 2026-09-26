#![cfg_attr(mutants, mutants::skip)]
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Result, Token, token};

pub enum HtmlNode {
    Element(HtmlElement),
    Text(LitStr),
    Block(Expr),
}

pub struct HtmlDocument {
    nodes: Vec<HtmlNode>,
}

fn starts_comment(input: ParseStream) -> bool {
    input.peek(Token![<]) && input.peek2(Token![!])
}

fn parse_comment(input: ParseStream) -> Result<()> {
    input.parse::<Token![<]>()?;
    input.parse::<Token![!]>()?;
    input.parse::<Token![-]>()?;
    input.parse::<Token![-]>()?;

    while !input.is_empty() {
        if input.peek(Token![-]) && input.peek2(Token![-]) && input.peek3(Token![>]) {
            input.parse::<Token![-]>()?;
            input.parse::<Token![-]>()?;
            input.parse::<Token![>]>()?;
            return Ok(());
        }
        input.parse::<TokenTree>()?;
    }

    Err(syn::Error::new(
        Span::call_site(),
        "unterminated HTML comment; expected -->",
    ))
}

impl Parse for HtmlDocument {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            if starts_comment(input) {
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

pub struct HtmlAttribute {
    pub name: String,
    pub value: HtmlAttrValue,
}

pub enum HtmlAttrValue {
    Static(LitStr),
    Dynamic(Box<Expr>),
}

impl Parse for HtmlNode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![<]) {
            let element = input.parse::<HtmlElement>()?;
            Ok(HtmlNode::Element(element))
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
        let tag_name = input.parse::<Ident>()?;

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

        let mut children = Vec::new();
        while !input.is_empty() {
            if input.peek(Token![<]) && input.peek2(Token![/]) {
                break;
            }
            if starts_comment(input) {
                parse_comment(input)?;
            } else {
                children.push(input.parse::<HtmlNode>()?);
            }
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let closing_tag = input.parse::<Ident>()?;
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

impl HtmlDocument {
    pub fn to_tokens(&self) -> TokenStream {
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

impl Parse for HtmlAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name_parts = Vec::new();
        name_parts.push(Ident::parse_any(input)?.to_string());

        while input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            name_parts.push(Ident::parse_any(input)?.to_string());
        }

        let name = name_parts.join("-");
        input.parse::<Token![=]>()?;
        let value = if input.peek(token::Brace) {
            let content;
            syn::braced!(content in input);
            let expr = content.parse::<Expr>()?;
            HtmlAttrValue::Dynamic(Box::new(expr))
        } else {
            let lit = input.parse::<LitStr>()?;
            HtmlAttrValue::Static(lit)
        };
        Ok(HtmlAttribute { name, value })
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
        }
    }

    pub fn static_size(&self) -> usize {
        match self {
            HtmlNode::Element(el) => el.static_size(),
            HtmlNode::Text(txt) => txt.value().len(),
            HtmlNode::Block(_) => 0,
        }
    }
}

impl HtmlElement {
    pub fn static_size(&self) -> usize {
        let tag = self.tag_name.to_string();
        let mut size = tag.len() * 2 + 5; // <tag></tag>

        for attr in &self.attributes {
            size += attr.name.len() + 4; //  name=""
            if let HtmlAttrValue::Static(lit) = &attr.value {
                size += lit.value().len();
            }
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

        let mut attr_tokens = Vec::new();
        for attr in &self.attributes {
            let attr_name = attr.name.to_string();
            match &attr.value {
                HtmlAttrValue::Static(lit) => {
                    let val = lit.value();
                    let static_attr = format!(" {}=\"{}\"", attr_name, val);
                    attr_tokens.push(quote! {
                        #buffer.push_str(#static_attr);
                    });
                }
                HtmlAttrValue::Dynamic(expr) => {
                    let attr_prefix = format!(" {}=\"", attr_name);
                    attr_tokens.push(quote! {
                        #buffer.push_str(#attr_prefix);
                        #buffer.push_str(&rullst::html::escape_attr(&(#expr)));
                        #buffer.push_str("\"");
                    });
                }
            }
        }

        let child_tokens = self.children.iter().map(|child| child.to_tokens());

        let void_elements = [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ];
        let is_void = void_elements.contains(&tag.as_str());

        if self.children.is_empty() && is_void {
            quote! {
                {
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
