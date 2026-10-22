//! History and alerts pages, and the forms that change watch, rules and
//! webhooks (phase 3).

use super::alerts::AlertRules;
use super::monitoring::unix_now;
use super::pages::{self, Page};
use super::projects::{ProjectError, WebhookChange};
use super::router::{
    ProjectQuery, RemoteState, form_token_valid, names, render, select, storage_error,
};
use crate::access::VerifiedLocalStudioAccess;
use axum::extract::{Extension, Form, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use std::sync::Arc;

/// Alerts listed on the page.
const LISTED_ALERTS: u32 = 100;

pub(crate) async fn history_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<ProjectQuery>,
) -> Response {
    let (project, all) = match select(&state, &query, Page::History) {
        Ok(selected) => selected,
        Err(response) => return *response,
    };
    let now = unix_now();
    let range = pages::history::Range::parse(query.range.as_deref());
    let (samples, available) = match &state.history {
        Some(history) => (
            history
                .samples(&project.name, now.saturating_sub(range.seconds()))
                .await
                .unwrap_or_default(),
            true,
        ),
        None => (Vec::new(), false),
    };
    let content = pages::history::page(&project.name, &samples, range, now, available);
    render(
        &state,
        &all,
        Some(&project.name),
        Page::History,
        "History",
        &content,
    )
    .into_response()
}

async fn alerts_response(
    state: &RemoteState,
    status: StatusCode,
    message: Option<(&str, &str, &str)>,
) -> Response {
    let projects = match state.store.load() {
        Ok(projects) => projects,
        Err(error) => return storage_error(state, error),
    };
    let alerts = match &state.history {
        Some(history) => {
            let count = history.unacknowledged().await.unwrap_or(0);
            state.monitoring.set_open_alerts(count);
            history.alerts(LISTED_ALERTS).await.ok()
        }
        None => None,
    };
    let content = pages::alerts::page(
        alerts.as_deref(),
        &projects,
        &state.form_token,
        message,
        state.poll_interval.as_secs(),
    );
    (
        status,
        render(
            state,
            &names(&projects),
            None,
            Page::Alerts,
            "Alerts",
            &content,
        ),
    )
        .into_response()
}

pub(crate) async fn alerts_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<ProjectQuery>,
) -> Response {
    let message = match query.notice.as_deref() {
        Some("saved") => Some((
            "ok",
            "Settings saved",
            "The webhook, if any, is encrypted on this machine.",
        )),
        Some("acknowledged") => Some(("ok", "Alerts acknowledged", "")),
        _ => None,
    };
    alerts_response(&state, StatusCode::OK, message).await
}

#[derive(Deserialize)]
pub(crate) struct SettingsForm {
    form_token: String,
    name: String,
    watch: Option<String>,
    unreachable: Option<String>,
    #[serde(default)]
    error_rate_percent: String,
    #[serde(default)]
    p95_ms: String,
    #[serde(default)]
    queue_pending: String,
    #[serde(default)]
    queue_failures: String,
    #[serde(default)]
    cooldown_minutes: String,
    #[serde(default)]
    webhook: String,
    clear_webhook: Option<String>,
}

/// An empty field disables the rule; anything unparsable is an error.
fn field<T: std::str::FromStr>(value: &str) -> Result<Option<T>, ProjectError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| ProjectError::InvalidRules)
}

pub(crate) fn parse_rules(form: &SettingsForm) -> Result<AlertRules, ProjectError> {
    let rules = AlertRules {
        unreachable: form.unreachable.is_some(),
        error_rate_percent: field(&form.error_rate_percent)?,
        p95_ms: field(&form.p95_ms)?,
        queue_pending: field(&form.queue_pending)?,
        queue_failures: field(&form.queue_failures)?,
        cooldown_minutes: field(&form.cooldown_minutes)?
            .unwrap_or(AlertRules::default().cooldown_minutes),
    };
    rules
        .valid()
        .then_some(rules)
        .ok_or(ProjectError::InvalidRules)
}

fn refused_response(state: &RemoteState) -> impl std::future::Future<Output = Response> + '_ {
    alerts_response(
        state,
        StatusCode::FORBIDDEN,
        Some((
            "error",
            "The form was refused",
            "Reload the page and submit it again from this Studio.",
        )),
    )
}

pub(crate) async fn save_settings(
    State(state): State<Arc<RemoteState>>,
    marker: Option<Extension<VerifiedLocalStudioAccess>>,
    Form(form): Form<SettingsForm>,
) -> Response {
    if marker.is_none() || !form_token_valid(&state.form_token, &form.form_token) {
        return refused_response(&state).await;
    }
    let webhook = if form.clear_webhook.is_some() {
        WebhookChange::Clear
    } else if form.webhook.trim().is_empty() {
        WebhookChange::Keep
    } else {
        WebhookChange::Set(form.webhook.trim().to_string())
    };
    let saved = parse_rules(&form).and_then(|rules| {
        state
            .store
            .update_settings(&form.name, form.watch.is_some(), rules, webhook)
    });
    match saved {
        Ok(()) => Redirect::to("/studio/alerts?notice=saved").into_response(),
        Err(error) => {
            alerts_response(
                &state,
                StatusCode::UNPROCESSABLE_ENTITY,
                Some(("error", "The settings were not saved", &error.to_string())),
            )
            .await
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct TokenForm {
    form_token: String,
}

pub(crate) async fn acknowledge(
    State(state): State<Arc<RemoteState>>,
    marker: Option<Extension<VerifiedLocalStudioAccess>>,
    Form(form): Form<TokenForm>,
) -> Response {
    if marker.is_none() || !form_token_valid(&state.form_token, &form.form_token) {
        return refused_response(&state).await;
    }
    if let Some(history) = &state.history
        && history.acknowledge_all().await.is_ok()
    {
        state.monitoring.set_open_alerts(0);
    }
    Redirect::to("/studio/alerts?notice=acknowledged").into_response()
}
