//! The CRUD handlers show and read `DateTimeUtc` values in the panel's zone.

use super::*;
use crate::nexus::types::{FieldKind, FieldMeta, NexusModel};
use crate::nexus::{Nexus, NexusAuditPolicy, NexusVerifiedTls};
use axum::body::Body;
use axum::http::{HeaderMap, Request};
use tower::ServiceExt;

fn events() -> RegistryEntry {
    RegistryEntry {
        table: "events",
        label: "Events",
        icon: "E",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields: vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("starts_at", "Starts at", FieldKind::DateTimeUtc),
        ],
    }
}

fn state(entry: RegistryEntry) -> State<Arc<NexusState>> {
    State(Arc::new(NexusState {
        registry: Arc::new(vec![entry]),
        brand: Arc::new("Nexus".to_owned()),
        audit_policy: NexusAuditPolicy::Disabled,
    }))
}

fn brasilia() -> Option<Extension<NexusTimeZone>> {
    Some(Extension(NexusTimeZone::fixed_offset(-180).unwrap()))
}

fn form(pairs: &[(&str, &str)]) -> axum::extract::Form<Vec<(String, String)>> {
    axum::extract::Form(
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
    )
}

async fn body(response: Response) -> (StatusCode, String) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[test]
fn the_panel_zone_defaults_to_utc() {
    assert_eq!(panel_zone(None), NexusTimeZone::utc());
    assert_eq!(
        panel_zone(brasilia()),
        NexusTimeZone::fixed_offset(-180).unwrap()
    );
}

#[tokio::test]
async fn the_new_form_names_the_panel_zone() {
    let (status, html) =
        body(nexus_new_form(state(events()), Path("events".to_owned()), None, brasilia()).await)
            .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains("<label class=\"nexus-label\">Starts at (UTC-03:00)</label>"),
        "{html}"
    );
    let (_, html) =
        body(nexus_new_form(state(events()), Path("events".to_owned()), None, None).await).await;
    assert!(html.contains("Starts at (UTC)</label>"), "{html}");

    let (status, _) =
        body(nexus_new_form(state(events()), Path("missing".to_owned()), None, None).await).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let mut tenant_scoped = events();
    tenant_scoped.tenant_column = Some("tenant_id");
    let (status, _) = body(
        nexus_new_form(
            state(tenant_scoped),
            Path("events".to_owned()),
            None,
            brasilia(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_edit_form_still_refuses_a_key_that_names_no_record() {
    let (status, html) = body(
        nexus_edit_form(
            state(events()),
            Path(("events".to_owned(), "+7".to_owned())),
            None,
            brasilia(),
        )
        .await,
    )
    .await;
    assert_eq!(
        (status, html.as_str()),
        (StatusCode::NOT_FOUND, "Record not found.")
    );
}

/// These updates end before a query runs: no database is configured.
#[tokio::test]
async fn updates_read_wall_times_in_the_panel_zone_before_validation() {
    let update = |zone, pairs: &[(&str, &str)]| {
        nexus_update_record(
            state(events()),
            Path(("events".to_owned(), "7".to_owned())),
            Extension(crate::nexus::NexusPrincipal::authenticated("test-admin")),
            None,
            zone,
            HeaderMap::new(),
            form(pairs),
        )
    };
    // 00:30 on 1 January of year 1 at UTC+01:00 is in year 0 in UTC.
    let ahead = Some(Extension(NexusTimeZone::fixed_offset(60).unwrap()));
    let (status, message) = body(update(ahead, &[("starts_at", "0001-01-01T00:30")]).await).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        message,
        "Invalid Events form: field `starts_at` must use a valid date-time; one without an offset is in the panel's time zone"
    );
    let (status, message) = body(update(None, &[("unknown", "x")]).await).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        message,
        "Invalid Events form: the form contains an unknown or protected field"
    );
}

#[cfg(feature = "tz")]
#[tokio::test]
async fn an_update_in_a_daylight_saving_gap_is_refused() {
    let new_york = NexusTimeZone::named("America/New_York").unwrap();
    let response = nexus_update_record(
        state(events()),
        Path(("events".to_owned(), "7".to_owned())),
        Extension(crate::nexus::NexusPrincipal::authenticated("test-admin")),
        None,
        Some(Extension(new_york)),
        HeaderMap::new(),
        form(&[("starts_at", "2026-03-08T02:30")]),
    )
    .await;
    let (status, message) = body(response).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        message,
        "Invalid Events form: field `starts_at` names a time that does not exist in America/New_York: clocks skip it at a daylight-saving change"
    );
}

struct EventModel;

impl NexusModel for EventModel {
    fn nexus_table() -> &'static str {
        "events"
    }
    fn nexus_label() -> &'static str {
        "Events"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        events().fields
    }
}

#[tokio::test]
async fn the_built_panel_uses_the_selected_zone() {
    use base64::Engine;

    let password = format!(
        "nexus_zone_test_{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let router = Nexus::new()
        .with_auth("admin", &password)
        .register::<EventModel>()
        .with_time_zone(NexusTimeZone::fixed_offset(-180).unwrap())
        .try_build()
        .expect("valid Nexus configuration");
    let mut request = Request::builder()
        .uri("/table/events/new")
        .header(
            axum::http::header::AUTHORIZATION,
            format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD.encode(format!("admin:{password}"))
            ),
        )
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        "192.0.2.30:443"
            .parse::<std::net::SocketAddr>()
            .expect("valid test peer"),
    ));
    request
        .extensions_mut()
        .insert(NexusVerifiedTls::from_trusted_tls_termination());
    let (status, html) = body(router.oneshot(request).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{html}");
    assert!(html.contains("Starts at (UTC-03:00)</label>"), "{html}");
}

#[test]
fn stored_instants_render_as_wall_times_of_the_panel_zone() {
    use crate::nexus::crud::form::{StoredValue, render_field_widget};
    let field = FieldMeta::new("starts_at", "Starts at", FieldKind::DateTimeUtc);
    let stored = StoredValue::Value("2026-10-15T12:30:00+00:00".to_owned());
    let brasilia = NexusTimeZone::fixed_offset(-180).unwrap();
    let html = render_field_widget(&field, &stored, true, &["id"], &brasilia);
    assert!(
        html.contains("type=\"datetime-local\" name=\"starts_at\" value=\"2026-10-15T09:30:00\""),
        "{html}"
    );
    let html = render_field_widget(&field, &stored, true, &["id"], &NexusTimeZone::utc());
    assert!(html.contains("value=\"2026-10-15T12:30:00\""), "{html}");
}
