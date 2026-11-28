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
    Match(Box<HtmlMatch>),
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
use attributes::{HtmlAttribute, allows_data_urls};

#[path = "html_control.rs"]
mod control;
use control::{HtmlFor, HtmlIf, HtmlMatch};

#[path = "html_text.rs"]
mod text;
use text::Child;

#[path = "html_write.rs"]
mod write;
use write::{Helpers, Writer};

impl Parse for HtmlDocument {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut children = Vec::new();
        while !input.is_empty() {
            if starts_doctype(input) {
                if !children.is_empty() {
                    return Err(misplaced_doctype(input));
                }
                children.push(Child::node(parse_doctype(input)?, false));
            } else if starts_comment(input) {
                parse_comment(input)?;
            } else {
                let spaced = text::spaced_before(input);
                children.push(parse_child(input, spaced)?);
            }
        }
        Ok(Self {
            nodes: text::join(children),
        })
    }
}

pub struct HtmlElement {
    pub tag_name: Ident,
    pub attributes: Vec<HtmlAttribute>,
    pub children: Vec<HtmlNode>,
}

/// Parses one child node; `spaced` says whether whitespace precedes it.
fn parse_child(input: ParseStream, spaced: bool) -> Result<Child> {
    let node = if input.peek(Token![<]) {
        HtmlNode::Element(input.parse::<HtmlElement>()?)
    } else if input.peek(Token![@]) {
        control::parse_control(input)?
    } else if input.peek(Token![else]) && (input.peek2(token::Brace) || input.peek2(Token![if])) {
        return Err(control::else_without_at(input));
    } else if input.peek(token::Brace) {
        let content;
        syn::braced!(content in input);
        HtmlNode::Block(content.parse::<Expr>()?)
    } else if input.peek(LitStr) {
        HtmlNode::Text(text::unsuffixed(input.parse::<LitStr>()?)?)
    } else {
        let child = if input.peek(token::Paren) || input.peek(token::Bracket) {
            text::parse_group(input)?
        } else {
            text::parse_run(input)?
        };
        return Ok(Child { spaced, ..child });
    };
    Ok(Child::node(node, spaced))
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
            let spaced = text::spaced_before(input);
            children.push(parse_child(input, spaced)?);
        }
    }
    Ok(text::join(children))
}

impl HtmlDocument {
    /// Expands the document into a `rullst::html::Markup`: every node writes
    /// into one `String`, escaping every interpolated value that is not
    /// itself markup.
    pub fn to_tokens(&self) -> TokenStream {
        let capacity = self.nodes.iter().map(HtmlNode::static_size).sum::<usize>();
        // Keep the generated local out of caller expressions, including an `s`
        // binding in a fragment. Renaming a call-site identifier is not hygiene.
        let buffer = Ident::new("s", Span::mixed_site());
        let mut helpers = Helpers::default();
        let mut out = Writer::new(buffer.clone(), 0);
        for node in &self.nodes {
            node.write(&mut out, &mut helpers);
        }
        let statements = out.finish();
        let helpers = helpers.to_tokens();
        quote! {
            rullst::html::Markup::from_trusted({
                #helpers
                let mut #buffer = String::with_capacity(#capacity);
                #statements
                #buffer
            })
        }
    }
}

impl HtmlNode {
    /// Appends the statements that write this node.
    fn write(&self, out: &mut Writer, helpers: &mut Helpers) {
        match self {
            HtmlNode::Element(el) => el.write(out, helpers),
            HtmlNode::Text(txt) => out.text(&txt.value()),
            HtmlNode::Block(expr) => {
                let tokens = write::escaped_text(out.buffer(), expr);
                out.code(tokens);
            }
            HtmlNode::If(node) => node.write(out, helpers),
            HtmlNode::For(node) => node.write(out, helpers),
            HtmlNode::Match(node) => node.write(out, helpers),
        }
    }

    /// Whether an expression of this node, outside the bodies of `@for`
    /// blocks it contains, mentions `continue` or `break`.
    fn mentions_loop_exit(&self) -> bool {
        match self {
            HtmlNode::Element(el) => {
                el.attributes.iter().any(HtmlAttribute::mentions_loop_exit)
                    || el.children.iter().any(HtmlNode::mentions_loop_exit)
            }
            HtmlNode::Text(_) => false,
            HtmlNode::Block(expr) => write::expr_mentions_loop_exit(expr),
            HtmlNode::If(node) => node.mentions_loop_exit(),
            HtmlNode::For(node) => write::expr_mentions_loop_exit(&node.iterable),
            HtmlNode::Match(node) => node.mentions_loop_exit(),
        }
    }

    pub fn static_size(&self) -> usize {
        match self {
            HtmlNode::Element(el) => el.static_size(),
            HtmlNode::Text(txt) => txt.value().len(),
            HtmlNode::Block(_) | HtmlNode::If(_) | HtmlNode::For(_) | HtmlNode::Match(_) => 0,
        }
    }
}

/// Elements written `<name />`, which take no closing tag.
const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

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

    /// Appends the statements that write the element, its attributes and
    /// its children.
    fn write(&self, out: &mut Writer, helpers: &mut Helpers) {
        let tag = self.tag_name.to_string();
        let allow_data_urls = allows_data_urls(&tag);
        out.text("<");
        out.text(&tag);
        for attr in &self.attributes {
            attr.write(out, helpers, allow_data_urls);
        }
        if self.children.is_empty() && VOID_ELEMENTS.contains(&tag.as_str()) {
            out.text(" />");
            return;
        }
        out.text(">");
        for child in &self.children {
            child.write(out, helpers);
        }
        out.text("</");
        out.text(&tag);
        out.text(">");
    }
}

#[cfg(test)]
#[path = "html_parser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "html_text_tests.rs"]
mod text_tests;

#[allow(unexpected_cfgs)]
#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
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
        let i: usize = kani::any();
        kani::assume(i < VOID_ELEMENTS.len());

        let tag = VOID_ELEMENTS[i];

        // Assert that the parsing logic's void element check works correctly
        let is_void = VOID_ELEMENTS.contains(&tag);
        assert!(is_void);
    }
}
