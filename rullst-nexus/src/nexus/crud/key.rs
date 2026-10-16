//! Record addresses: how a URL, a batch selection and the audit name one
//! record, for single-column and composite primary keys.
//!
//! A single-column key is addressed by its value, as before composite keys
//! existed. A composite key is addressed by its values in key order, each
//! percent-encoded with `urlencoding::encode` (every byte except ASCII letters,
//! digits and `-_.~`) and joined by `,`. Encoding always escapes `,`, so the
//! split is unambiguous, and only the canonical spelling is accepted, so one
//! record has exactly one address (the one the audit records).

use std::borrow::Cow;

use super::dialect::{NexusQuery, placeholder};
use super::query::sanitize_identifier;
use crate::nexus::types::{FieldKind, RegistryEntry};

/// Joins the encoded values of a composite key.
const KEY_SEPARATOR: char = ',';

/// One typed primary-key value.
#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyValue<'a> {
    /// A `number`, `integer` or relation column: bound as a 64-bit integer.
    Integer(i64),
    /// Any other column: compared as text, so a text key such as `1001` is
    /// never bound as an integer.
    Text(Cow<'a, str>),
}

/// A record address from a URL or batch form, typed by the registered key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordKey<'a> {
    address: &'a str,
    values: Vec<KeyValue<'a>>,
}

impl<'a> RecordKey<'a> {
    /// Parses `address` as the model's primary key requires: one value per
    /// key column, each canonical for its column. An integer column needs a
    /// canonical integer: `+1`, `01` or `1e3` would otherwise reach a record
    /// whose stored key is spelled differently (Rust integer parsing and
    /// SQLite numeric affinity both accept them), and the audit would name a
    /// key that differs from the changed record. `None` means `address`
    /// cannot name a record of this model.
    pub(crate) fn parse(entry: &RegistryEntry, address: &'a str) -> Option<Self> {
        let values = match entry.key_columns() {
            [column] => vec![key_value(entry, column, Cow::Borrowed(address))?],
            columns => {
                let parts = address.split(KEY_SEPARATOR).collect::<Vec<_>>();
                if parts.len() != columns.len() {
                    return None;
                }
                columns
                    .iter()
                    .zip(parts)
                    .map(|(column, part)| key_value(entry, column, decode_part(part)?))
                    .collect::<Option<Vec<_>>>()?
            }
        };
        Some(Self { address, values })
    }

    /// The canonical address, as the URL, the batch form and the audit use it.
    pub(crate) fn text(&self) -> &'a str {
        self.address
    }

    /// Binds every key value in key order, each as its registered kind.
    pub(crate) fn bind<'q>(&self, mut query: NexusQuery<'q>) -> NexusQuery<'q> {
        for value in &self.values {
            query = match value {
                KeyValue::Integer(value) => query.bind(*value),
                KeyValue::Text(value) => query.bind(value.clone().into_owned()),
            };
        }
        query
    }
}

/// One key value of `column`, typed by its registered field kind. A key
/// column that is not registered (only possible for a hand-built entry)
/// keeps the integer fallback for canonical integers.
fn key_value<'a>(entry: &RegistryEntry, column: &str, text: Cow<'a, str>) -> Option<KeyValue<'a>> {
    let canonical = text
        .parse::<i64>()
        .ok()
        .filter(|value| value.to_string() == *text);
    let integer_key = entry
        .fields
        .iter()
        .find(|field| field.name == column)
        .map(|field| {
            matches!(
                field.kind,
                FieldKind::Number | FieldKind::Integer { .. } | FieldKind::ForeignKey { .. }
            )
        });
    match (integer_key, canonical) {
        (Some(true) | None, Some(value)) => Some(KeyValue::Integer(value)),
        (Some(true), None) => None,
        (Some(false) | None, None) | (Some(false), Some(_)) => Some(KeyValue::Text(text)),
    }
}

/// Decodes one composite-key value, accepting only its canonical encoding.
fn decode_part(part: &str) -> Option<Cow<'_, str>> {
    let decoded = urlencoding::decode(part).ok()?;
    (urlencoding::encode(&decoded) == part).then_some(decoded)
}

/// The address of a record whose key columns hold `values`, in key order.
pub(crate) fn record_address(values: &[String]) -> String {
    match values {
        [value] => value.clone(),
        _ => values
            .iter()
            .map(|value| urlencoding::encode(value))
            .collect::<Vec<_>>()
            .join(","),
    }
}

/// `column = placeholder` for every key column, joined by `AND`, with the
/// placeholders numbered from `first` in key order.
pub(crate) fn key_predicate(entry: &RegistryEntry, first: usize, driver: &str) -> String {
    entry
        .key_columns()
        .iter()
        .enumerate()
        .map(|(index, column)| {
            format!(
                "{} = {}",
                sanitize_identifier(column),
                placeholder(first + index, driver)
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

/// Matches `count` records by key, with placeholders numbered from 1: `pk IN
/// (…)` for a single key, `((a = ? AND b = ?) OR …)` for a composite key.
pub(crate) fn keys_predicate(entry: &RegistryEntry, count: usize, driver: &str) -> String {
    let width = entry.key_columns().len();
    match entry.key_columns() {
        [column] => format!(
            "{} IN ({})",
            sanitize_identifier(column),
            (1..=count)
                .map(|position| placeholder(position, driver))
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => format!(
            "({})",
            (0..count)
                .map(|record| format!("({})", key_predicate(entry, record * width + 1, driver)))
                .collect::<Vec<_>>()
                .join(" OR ")
        ),
    }
}

#[cfg(test)]
#[path = "key_tests.rs"]
mod tests;
