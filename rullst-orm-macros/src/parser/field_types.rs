//! Field type shapes the model parser recognizes.

use super::EncryptedFieldKind;

pub(super) fn encrypted_field_kind(field_type: &syn::Type) -> Option<EncryptedFieldKind> {
    let syn::Type::Path(type_path) = field_type else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident == "String" && matches!(segment.arguments, syn::PathArguments::None) {
        return Some(EncryptedFieldKind::String);
    }
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    if arguments.args.len() != 1 {
        return None;
    }
    let syn::GenericArgument::Type(syn::Type::Path(inner_path)) = &arguments.args[0] else {
        return None;
    };
    let inner = inner_path.path.segments.last()?;
    (inner.ident == "String" && matches!(inner.arguments, syn::PathArguments::None))
        .then_some(EncryptedFieldKind::OptionalString)
}

/// Recognizes `SecretString` and `Option<SecretString>` by the last path segment.
pub(crate) fn is_secret_string_type(field_type: &syn::Type) -> bool {
    let syn::Type::Path(type_path) = field_type else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    match (&segment.arguments, segment.ident == "Option") {
        (syn::PathArguments::None, false) => segment.ident == "SecretString",
        (syn::PathArguments::AngleBracketed(arguments), true) => {
            matches!(arguments.args.first(), Some(syn::GenericArgument::Type(inner))
                if arguments.args.len() == 1 && !is_option(inner) && is_secret_string_type(inner))
        }
        _ => false,
    }
}

fn is_option(field_type: &syn::Type) -> bool {
    matches!(field_type, syn::Type::Path(path)
        if path.path.segments.last().is_some_and(|segment| segment.ident == "Option"))
}
