use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitStr, Result, Token, token};

pub enum HtmlNode {
    Element(HtmlElement),
    Text(LitStr),
    Block(Expr),
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
    Dynamic(Expr),
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
            children.push(input.parse::<HtmlNode>()?);
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
            HtmlAttrValue::Dynamic(expr)
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

        let mut attr_tokens = Vec::new();
        for attr in &self.attributes {
            let attr_name = attr.name.to_string();
            match &attr.value {
                HtmlAttrValue::Static(lit) => {
                    let val = lit.value();
                    attr_tokens.push(quote! {
                        format!(" {}=\"{}\"", #attr_name, #val)
                    });
                }
                HtmlAttrValue::Dynamic(expr) => {
                    attr_tokens.push(quote! {
                        format!(" {}=\"{}\"", #attr_name, rullst::html::escape_attr(&(#expr)))
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
                    let mut s = String::with_capacity(#capacity);
                    s.push_str("<");
                    s.push_str(#tag);
                    #( s.push_str(&#attr_tokens); )*
                    s.push_str(" />");
                    s
                }
            }
        } else {
            quote! {
                {
                    let mut s = String::with_capacity(#capacity);
                    s.push_str("<");
                    s.push_str(#tag);
                    #( s.push_str(&#attr_tokens); )*
                    s.push_str(">");
                    #( s.push_str(&#child_tokens); )*
                    s.push_str("</");
                    s.push_str(#tag);
                    s.push_str(">");
                    s
                }
            }
        }
    }
}
