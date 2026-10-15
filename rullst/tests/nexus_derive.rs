#![cfg(all(feature = "nexus", feature = "orm"))]

use rullst::db::Nexus;
use rullst::nexus::{FieldKind, NexusModel};

#[derive(Nexus)]
#[allow(dead_code)]
#[nexus(
    table = "derived_articles",
    label = "Editorial Articles",
    icon = "📰",
    primary_key = "uuid"
)]
struct DerivedArticle {
    uuid: String,
    #[nexus(kind = "textarea", label = "Article body")]
    body: String,
    #[nexus(kind = "enum", options = "draft, published")]
    status: String,
    is_active: bool,
}

/// Integer fields are bounded by their Rust type, so Nexus never stores a
/// fraction or an out-of-range value the model could not decode.
#[derive(Nexus)]
#[allow(dead_code)]
#[nexus(table = "stock_items")]
struct StockItem {
    id: i64,
    stock: i32,
    views: Option<u32>,
    total: u64,
    price: f64,
}

#[derive(Nexus)]
#[allow(dead_code)]
#[nexus(table = "tenant_articles", tenant = "organization_id")]
struct TenantArticle {
    id: i64,
    organization_id: String,
    title: String,
}

/// A real ORM model: Nexus must follow its skip and confidentiality markers.
#[derive(Clone, Debug, Default, rullst::db::Orm, rullst::db::FromRow, Nexus)]
#[allow(dead_code)]
#[orm(table = "nexus_patients")]
struct NexusPatient {
    id: i32,
    name: String,
    #[sqlx(default, skip)]
    display_name: String,
    #[orm(hidden)]
    reset_token: String,
    #[orm(encrypted)]
    diagnosis: String,
    cpf: Option<rullst::db::SecretString>,
    #[orm(masked)]
    api_token: String,
}

#[test]
fn derive_nexus_follows_orm_field_semantics() {
    let fields = NexusPatient::nexus_fields();
    assert!(fields.iter().all(|field| field.name != "display_name"));
    for sealed in ["reset_token", "diagnosis", "cpf"] {
        let field = fields
            .iter()
            .find(|field| field.name == sealed)
            .expect("protected field metadata");
        assert!(field.hidden && field.readonly, "{sealed}");
        assert_eq!(field.kind, FieldKind::Password, "{sealed}");
    }
    let masked = fields
        .iter()
        .find(|field| field.name == "api_token")
        .expect("masked field metadata");
    assert_eq!(masked.kind, FieldKind::Password);
    assert!(!masked.hidden);
    let name = fields
        .iter()
        .find(|field| field.name == "name")
        .expect("plain field metadata");
    assert_eq!(name.kind, FieldKind::Text);
}

#[test]
fn derive_nexus_generates_model_and_widget_metadata() {
    assert_eq!(DerivedArticle::nexus_table(), "derived_articles");
    assert_eq!(DerivedArticle::nexus_label(), "Editorial Articles");
    assert_eq!(DerivedArticle::nexus_icon(), "📰");
    assert_eq!(DerivedArticle::nexus_pk(), "uuid");

    let fields = DerivedArticle::nexus_fields();
    assert_eq!(fields.len(), 4);
    // An application-assigned (non-integer) key is entered on create; the
    // runtime keeps it read-only on edit.
    assert!(!fields[0].hidden && !fields[0].readonly);
    assert_eq!(fields[1].label, "Article body");
    assert_eq!(fields[1].kind, FieldKind::Textarea);
    assert_eq!(
        fields[2].kind,
        FieldKind::Enum {
            options: vec!["draft", "published"]
        }
    );
    assert_eq!(fields[3].kind, FieldKind::Boolean);
}

#[test]
fn derive_nexus_exposes_protected_tenant_metadata_through_the_facade() {
    assert_eq!(
        TenantArticle::nexus_tenant_column(),
        Some("organization_id")
    );
    let tenant = TenantArticle::nexus_fields()
        .into_iter()
        .find(|field| field.name == "organization_id")
        .expect("tenant field metadata");
    assert!(tenant.hidden && tenant.readonly);
    assert_eq!(tenant.kind, FieldKind::Text);
}

#[test]
fn derive_nexus_bounds_integer_fields_by_their_rust_type() {
    let fields = StockItem::nexus_fields();
    assert_eq!(
        fields[0].kind,
        FieldKind::Integer {
            min: i64::MIN,
            max: i64::MAX
        }
    );
    assert_eq!(
        fields[1].kind,
        FieldKind::Integer {
            min: i32::MIN.into(),
            max: i32::MAX.into()
        }
    );
    assert_eq!(
        fields[2].kind,
        FieldKind::Integer {
            min: 0,
            max: u32::MAX.into()
        }
    );
    assert_eq!(
        fields[3].kind,
        FieldKind::Integer {
            min: 0,
            max: i64::MAX
        }
    );
    assert_eq!(fields[4].kind, FieldKind::Number);
}

/// A composite key: both columns name the record and both are entered.
#[derive(Nexus)]
#[allow(dead_code)]
#[nexus(table = "lesson_progress", primary_key = "user_id, lesson_id")]
struct LessonProgress {
    user_id: i32,
    lesson_id: i32,
    percent: i32,
}

#[test]
fn derive_nexus_declares_composite_primary_keys() {
    assert_eq!(LessonProgress::nexus_pk(), "user_id");
    assert_eq!(LessonProgress::nexus_pk_columns(), ["user_id", "lesson_id"]);
    assert_eq!(DerivedArticle::nexus_pk_columns(), ["uuid"]);
    for field in LessonProgress::nexus_fields()
        .into_iter()
        .filter(|field| field.name != "percent")
    {
        assert!(!field.hidden && !field.readonly, "{}", field.name);
    }
    let _router = rullst::nexus::Nexus::new()
        .register::<LessonProgress>()
        .with_auth("composite-admin", "composite-key-test-credential")
        .try_build()
        .expect("a derived composite key passes registry validation");
}

/// Chrono fields get the matching date/time widget.
#[derive(Nexus)]
#[allow(dead_code)]
#[nexus(table = "appointments")]
struct Appointment {
    id: i64,
    due_on: rullst::orm::chrono::NaiveDate,
    opens_at: Option<rullst::orm::chrono::NaiveTime>,
    starts_at: rullst::orm::chrono::NaiveDateTime,
    paid_at: rullst::orm::chrono::DateTime<rullst::orm::chrono::Utc>,
}

#[test]
fn derive_nexus_maps_chrono_fields_to_date_and_time_widgets() {
    let kinds = Appointment::nexus_fields()
        .into_iter()
        .map(|field| (field.name, field.kind))
        .collect::<Vec<_>>();
    assert_eq!(kinds[1], ("due_on", FieldKind::Date));
    assert_eq!(kinds[2], ("opens_at", FieldKind::Time));
    assert_eq!(kinds[3], ("starts_at", FieldKind::DateTime));
    assert_eq!(kinds[4], ("paid_at", FieldKind::DateTimeUtc));
}
