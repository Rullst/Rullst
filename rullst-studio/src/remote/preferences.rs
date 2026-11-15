//! Studio-wide settings (`/studio/settings`) and the notification feed that
//! open Studio pages read (`/studio/notifications`).

use super::notify::NotificationSettings;
use super::pages::{self, Page};
use super::router::{RemoteState, form_token_valid, names, render, storage_error};
use crate::access::VerifiedLocalStudioAccess;
use axum::Json;
use axum::extract::{Extension, Form, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The settings that apply to the whole Studio, kept in the project list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StudioSettings {
    /// Desktop notifications of alerts.
    #[serde(default)]
    pub(crate) notifications: NotificationSettings,
}

impl StudioSettings {
    pub(crate) fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

async fn settings_response(
    state: &RemoteState,
    status: StatusCode,
    message: Option<(&str, &str, &str)>,
) -> Response {
    let (projects, settings) = match (state.store.load(), state.store.settings()) {
        (Ok(projects), Ok(settings)) => (projects, settings),
        (Err(error), _) | (_, Err(error)) => return storage_error(state, error),
    };
    let content = pages::preferences::page(&settings, &state.form_token, message);
    (
        status,
        render(
            state,
            &names(&projects),
            None,
            Page::Settings,
            "Settings",
            &content,
        ),
    )
        .into_response()
}

#[derive(Deserialize)]
pub(crate) struct SettingsQuery {
    notice: Option<String>,
}

pub(crate) async fn settings_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<SettingsQuery>,
) -> Response {
    let message = match query.notice.as_deref() {
        Some("saved") => Some(("ok", "Settings saved", "")),
        _ => None,
    };
    settings_response(&state, StatusCode::OK, message).await
}

#[derive(Deserialize)]
pub(crate) struct NotificationsForm {
    form_token: String,
    enabled: Option<String>,
    #[serde(default)]
    quiet_minutes: String,
}

pub(crate) async fn save_notifications(
    State(state): State<Arc<RemoteState>>,
    marker: Option<Extension<VerifiedLocalStudioAccess>>,
    Form(form): Form<NotificationsForm>,
) -> Response {
    if marker.is_none() || !form_token_valid(&state.form_token, &form.form_token) {
        return settings_response(
            &state,
            StatusCode::FORBIDDEN,
            Some((
                "error",
                "The form was refused",
                "Reload the page and submit it again from this Studio.",
            )),
        )
        .await;
    }
    let saved = form
        .quiet_minutes
        .trim()
        .parse()
        .map_err(|_| super::projects::ProjectError::InvalidSettings)
        .and_then(|quiet_minutes| {
            state.store.update_notifications(NotificationSettings {
                enabled: form.enabled.is_some(),
                quiet_minutes,
            })
        });
    match saved {
        Ok(()) => Redirect::to("/studio/settings?notice=saved").into_response(),
        Err(error) => {
            settings_response(
                &state,
                StatusCode::UNPROCESSABLE_ENTITY,
                Some(("error", "The settings were not saved", &error.to_string())),
            )
            .await
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct FeedQuery {
    session: Option<String>,
    after: Option<u64>,
}

#[derive(Serialize)]
struct Feed<'a> {
    enabled: bool,
    session: &'a str,
    cursor: u64,
    notifications: Vec<super::notify::Notification>,
}

/// The notifications a page has not shown yet; see [`super::notify`].
pub(crate) async fn notifications_feed(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<FeedQuery>,
) -> Response {
    let enabled = state
        .store
        .settings()
        .is_ok_and(|settings| settings.notifications.enabled);
    let (cursor, notifications) = state.notifier.since(query.session.as_deref(), query.after);
    Json(Feed {
        enabled,
        session: state.notifier.session(),
        cursor,
        notifications,
    })
    .into_response()
}
