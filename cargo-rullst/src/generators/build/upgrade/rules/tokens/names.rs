//! Names a file uses in code, read once per file, that decide a rule found
//! elsewhere in it. Like the rest of the token scan they never come from a
//! comment, a doc comment or an attribute.

use super::{is_group, is_punct, string_value};
use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// Names the file uses in code that decide a rule elsewhere in the file:
/// identifiers outside attributes, so comments, doc comments and string
/// literals never satisfy them, except where noted.
#[derive(Debug, Default, PartialEq, Eq)]
pub(in super::super) struct CodeNames {
    /// A `captions_url` field, or the `"captions_url"` name of a Nexus field
    /// (`V13-LMS-MEDIA-KINDS`).
    pub captions_url: bool,
    /// A `DatabaseError` variant or type (`V13-ORM-TRANSACTION-ERRORS`).
    pub database_error: bool,
    /// A `MAX_PAGE…` bound (`V13-PAGE-BOUNDS`).
    pub max_page: bool,
}

impl CodeNames {
    pub(in super::super) fn of(stream: TokenStream) -> Self {
        let mut names = Self::default();
        names.read(stream);
        names
    }

    fn read(&mut self, stream: TokenStream) {
        let tokens: Vec<TokenTree> = stream.into_iter().collect();
        let mut index = 0;
        while let Some(tree) = tokens.get(index) {
            index += 1;
            match tree {
                TokenTree::Punct(punct) if punct.as_char() == '#' => {
                    let next = index + usize::from(is_punct(tokens.get(index), '!'));
                    if is_group(tokens.get(next), Delimiter::Bracket) {
                        index = next + 1;
                    }
                }
                TokenTree::Ident(ident) => {
                    let name = ident.to_string();
                    self.captions_url |= name == "captions_url";
                    self.database_error |= name.contains("DatabaseError");
                    self.max_page |= name.contains("MAX_PAGE");
                }
                TokenTree::Literal(_) => {
                    self.captions_url |=
                        string_value(Some(tree)).is_some_and(|value| value == "captions_url");
                }
                TokenTree::Group(group) => self.read(group.stream()),
                TokenTree::Punct(_) => {}
            }
        }
    }
}
