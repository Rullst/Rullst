//! The account files of `cargo rullst auth` (and the SaaS starter) that the
//! generated second factor calls.
use crate::generators::consumer_files as writes;
use std::error::Error;
use std::io;
use std::path::Path;
use syn::{ImplItem, Item, Type, Visibility};

const USER: &str = "src/models/user.rs";
const MIDDLEWARE: &str = "src/middlewares/auth_middleware.rs";
/// The `User` fields and methods the account adapter uses.
const USER_PARTS: [&str; 5] = [
    "email",
    "session_epoch",
    "current_session_epoch",
    "find_by_email",
    "revoke_sessions",
];

/// `relative` parsed, `None` when it does not exist or is not valid Rust.
pub(super) fn parsed(root: &Path, relative: &str) -> io::Result<Option<syn::File>> {
    match writes::read(&root.join(relative)) {
        Ok(source) => Ok(syn::parse_file(&source).ok()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Whether the `User` struct has the field `name` or an impl of it the method.
pub(super) fn user_declares(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        Item::Struct(user) if user.ident == "User" => user
            .fields
            .iter()
            .any(|field| field.ident.as_ref().is_some_and(|ident| ident == name)),
        Item::Impl(block) => {
            matches!(&*block.self_ty, Type::Path(path) if path.path.is_ident("User"))
                && block
                    .items
                    .iter()
                    .any(|item| matches!(item, ImplItem::Fn(method) if method.sig.ident == name))
        }
        _ => false,
    })
}

/// Whether a top-level function `name` is visible outside its module.
pub(super) fn public_function(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| {
        matches!(item, Item::Fn(function)
            if function.sig.ident == name && !matches!(function.vis, Visibility::Inherited))
    })
}

/// What the project lacks of the starter's accounts; empty when it has them.
pub(super) fn missing(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut missing = Vec::new();
    match parsed(root, USER)? {
        None => missing.push(format!("{USER} with a `User` model")),
        Some(file) => {
            let absent = USER_PARTS
                .into_iter()
                .filter(|part| !user_declares(&file, part))
                .map(|part| format!("`{part}`"))
                .collect::<Vec<_>>();
            if !absent.is_empty() {
                missing.push(format!("{} in the `User` of {USER}", absent.join(", ")));
            }
        }
    }
    match parsed(root, MIDDLEWARE)? {
        Some(file) if public_function(&file, "auth_middleware") => {}
        _ => missing.push(format!("`pub async fn auth_middleware` in {MIDDLEWARE}")),
    }
    Ok(missing)
}
