use super::*;
use crate::nexus::types::FieldMeta;

fn entry(pk_kind: FieldKind) -> RegistryEntry {
    RegistryEntry {
        table: "records",
        label: "Records",
        icon: "R",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields: vec![FieldMeta::new("id", "ID", pk_kind)],
    }
}

/// `lesson_progress (user_id, lesson_id, attempt)`: integer, relation and text.
fn composite() -> RegistryEntry {
    RegistryEntry {
        table: "lesson_progress",
        label: "Progress",
        icon: "P",
        pk: "user_id",
        pk_columns: vec!["user_id", "lesson_id", "attempt"],
        tenant_column: None,
        fields: vec![
            FieldMeta::new("user_id", "User", FieldKind::Integer { min: 1, max: 99 }),
            FieldMeta::new(
                "lesson_id",
                "Lesson",
                FieldKind::ForeignKey {
                    table: "lessons",
                    label_col: "title",
                },
            ),
            FieldMeta::new("attempt", "Attempt", FieldKind::Text),
            FieldMeta::new("percent", "Percent", FieldKind::Number),
        ],
    }
}

fn values<'a>(key: &RecordKey<'a>) -> Vec<KeyValue<'a>> {
    key.values.clone()
}

fn text(value: &str) -> KeyValue<'_> {
    KeyValue::Text(Cow::Borrowed(value))
}

#[test]
fn integer_keys_are_canonical_integers() {
    let integer = entry(FieldKind::Integer {
        min: 0,
        max: i64::MAX,
    });
    let key = RecordKey::parse(&integer, "7").expect("canonical integer");
    assert_eq!(values(&key), [KeyValue::Integer(7)]);
    assert_eq!(key.text(), "7");
    assert_eq!(RecordKey::parse(&integer, "07"), None);
}

#[test]
fn numeric_keys_must_be_canonical_integers() {
    let numeric = entry(FieldKind::Number);
    let key = RecordKey::parse(&numeric, "1000").expect("canonical integer");
    assert_eq!(values(&key), [KeyValue::Integer(1000)]);
    assert_eq!(key.text(), "1000");
    let negative = RecordKey::parse(&numeric, "-7").expect("negative integer");
    assert_eq!(values(&negative), [KeyValue::Integer(-7)]);
    for spelling in ["+1000", "01000", "1e3", "1000.0", " 1000", "abc", ""] {
        assert_eq!(RecordKey::parse(&numeric, spelling), None, "{spelling:?}");
    }
}

#[test]
fn text_keys_are_never_bound_as_integers() {
    let text_key = entry(FieldKind::Text);
    let key = RecordKey::parse(&text_key, "1001").expect("text key");
    assert_eq!(values(&key), [text("1001")]);
    let key = RecordKey::parse(&text_key, "a,b").expect("a single key keeps commas");
    assert_eq!(values(&key), [text("a,b")]);
    assert_eq!(key.text(), "a,b");

    // An entry whose key is not registered keeps the integer fallback.
    let mut unregistered = entry(FieldKind::Text);
    unregistered.pk = "uuid";
    let key = RecordKey::parse(&unregistered, "12").expect("fallback integer");
    assert_eq!(values(&key), [KeyValue::Integer(12)]);
    let key = RecordKey::parse(&unregistered, "012").expect("fallback text");
    assert_eq!(values(&key), [text("012")]);
}

#[test]
fn composite_keys_need_one_canonical_value_per_column() {
    let entry = composite();
    let key = RecordKey::parse(&entry, "3,42,first").expect("valid composite key");
    assert_eq!(
        values(&key),
        [KeyValue::Integer(3), KeyValue::Integer(42), text("first")]
    );
    assert_eq!(key.text(), "3,42,first");

    // Encoded separators, spaces and non-ASCII text stay one value.
    let key = RecordKey::parse(&entry, "3,42,a%2Cb%20c%C3%A9").expect("encoded text");
    assert_eq!(values(&key)[2], text("a,b cé"));
    let key = RecordKey::parse(&entry, "3,42,").expect("an empty text value");
    assert_eq!(values(&key)[2], text(""));

    for invalid in [
        "3,42",
        "3,42,first,extra",
        "3",
        "",
        // Integer columns need canonical integers.
        "03,42,first",
        "3,+42,first",
        "x,42,first",
        // Only the canonical encoding names the record.
        "3,42,a b",
        "3,42,a+b",
        "3,42,%61",
        "3,42,a%2cb",
        "3,42,%zz",
        "3,42,%C3",
        "3,42,a%2",
    ] {
        assert_eq!(RecordKey::parse(&entry, invalid), None, "{invalid:?}");
    }
}

#[test]
fn record_addresses_round_trip_through_parse() {
    assert_eq!(record_address(&["a,b c".to_string()]), "a,b c");
    let parts = ["3".to_string(), "42".to_string(), "a,b cé/~".to_string()];
    let address = record_address(&parts);
    assert_eq!(address, "3,42,a%2Cb%20c%C3%A9%2F~");
    let entry = composite();
    let key = RecordKey::parse(&entry, &address).expect("own address parses");
    assert_eq!(
        values(&key),
        [
            KeyValue::Integer(3),
            KeyValue::Integer(42),
            text("a,b cé/~")
        ]
    );
}

#[test]
fn key_predicates_bind_every_key_column_in_order() {
    let single = entry(FieldKind::Number);
    assert_eq!(key_predicate(&single, 1, "sqlite"), "id = ?");
    assert_eq!(key_predicate(&single, 4, "postgres"), "id = $4");

    let composite = composite();
    assert_eq!(
        key_predicate(&composite, 1, "mysql"),
        "user_id = ? AND lesson_id = ? AND attempt = ?"
    );
    assert_eq!(
        key_predicate(&composite, 3, "postgres"),
        "user_id = $3 AND lesson_id = $4 AND attempt = $5"
    );
}

#[test]
fn batch_predicates_match_each_selected_key() {
    let single = entry(FieldKind::Number);
    assert_eq!(keys_predicate(&single, 3, "sqlite"), "id IN (?,?,?)");
    assert_eq!(keys_predicate(&single, 2, "postgres"), "id IN ($1,$2)");

    let mut pair = composite();
    pair.pk_columns = vec!["user_id", "lesson_id"];
    assert_eq!(
        keys_predicate(&pair, 1, "sqlite"),
        "((user_id = ? AND lesson_id = ?))"
    );
    assert_eq!(
        keys_predicate(&pair, 3, "postgres"),
        "((user_id = $1 AND lesson_id = $2) OR (user_id = $3 AND lesson_id = $4) \
         OR (user_id = $5 AND lesson_id = $6))"
    );
}
