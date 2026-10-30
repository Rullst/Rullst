//! Checks of the polymorphic relations a model declares.

use super::ParsedRelation;

/// Every polymorphic relation names `morph_name`, and a `morph_to` relation's
/// id and discriminator columns are persisted fields of bindable types.
pub(super) fn validate_polymorphic_relations(
    relations: &[ParsedRelation],
    normal_fields: &[syn::Ident],
    normal_fields_types: &[syn::Type],
) -> Result<(), syn::Error> {
    for relation in relations {
        if matches!(
            relation.rel_type.as_str(),
            "morph_many" | "morph_one" | "morph_to"
        ) && relation.morph_name.is_empty()
        {
            return Err(syn::Error::new(
                relation.field_name.span(),
                format!(
                    "polymorphic relation `{}` requires `morph_name = \"...\"` (the legacy alias `name` is also accepted)",
                    relation.field_name
                ),
            ));
        }

        if relation.rel_type != "morph_to" {
            continue;
        }

        let morph_id_column = if relation.foreign_key.is_empty() {
            format!("{}_id", relation.morph_name)
        } else {
            relation.foreign_key.clone()
        };
        let morph_type_column = format!("{}_type", relation.morph_name);
        let persisted_field = |column: &str| {
            normal_fields
                .iter()
                .zip(normal_fields_types.iter())
                .find(|(field, _)| *field == column)
        };

        let Some((morph_id_field, morph_id_type)) = persisted_field(&morph_id_column) else {
            return Err(syn::Error::new(
                relation.field_name.span(),
                format!(
                    "morph_to relation `{}` requires persisted id field `{}`",
                    relation.field_name, morph_id_column
                ),
            ));
        };
        let id_is_bindable = match morph_id_type {
            syn::Type::Path(path) if path.qself.is_none() => {
                path.path.segments.last().is_some_and(|segment| {
                    matches!(
                        segment.ident.to_string().as_str(),
                        "String" | "i32" | "f64" | "bool"
                    )
                })
            }
            _ => false,
        };
        if !id_is_bindable {
            return Err(syn::Error::new(
                morph_id_field.span(),
                "morph_to id fields support String, i32, f64, or bool so they can be bound without lossy conversion",
            ));
        }

        let Some((morph_type_field, morph_type_type)) = persisted_field(&morph_type_column) else {
            return Err(syn::Error::new(
                relation.field_name.span(),
                format!(
                    "morph_to relation `{}` requires persisted discriminator field `{}`",
                    relation.field_name, morph_type_column
                ),
            ));
        };
        let discriminator_is_string = match morph_type_type {
            syn::Type::Path(path) if path.qself.is_none() => path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "String"),
            _ => false,
        };
        if !discriminator_is_string {
            return Err(syn::Error::new(
                morph_type_field.span(),
                "morph_to discriminator fields must use String",
            ));
        }
    }
    Ok(())
}
