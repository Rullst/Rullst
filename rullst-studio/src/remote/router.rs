//! Routes of the remote mode. Only remote pages exist here: the data browser,
//! `.env` viewer, AI and API playgrounds and migration pages are not mounted.

use super::client::MonitorClient;
use super::pages::{self, Chrome, Page};
use super::projects::{Project, ProjectError, ProjectStore};
use crate::access::VerifiedLocalStudioAccess;
use axum::Router;
use axum::extract::{DefaultBodyLimit, Extension, Form, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use std::sync::Arc;

/// Largest accepted form body.
const MAX_FORM_BYTES: usize = 8 * 1024;

/// Shared state of the remote Studio.
pub(crate) struct RemoteState {
    pub(crate) store: ProjectStore,
    pub(crate) client: MonitorClient,
    /// Random per process; required by every form that changes state.
    pub(crate) form_token: String,
    pub(crate) monitoring: super::monitoring::Monitoring,
}

pub(crate) fn router(state: Arc<RemoteState>) -> Router {
    let router = Router::new()
        .route("/", get(|| async { Redirect::to("/studio") }))
        .route("/studio", get(projects_page))
        .route("/studio/projects", post(add_project))
        .route("/studio/projects/remove", post(remove_project))
        .route("/studio/radar", get(overview_page))
        .route("/studio/routes", get(routes_page))
        .route("/studio/queues", get(queues_page))
        .route("/studio/queries", get(queries_page))
        .layer(DefaultBodyLimit::max(MAX_FORM_BYTES))
        .with_state(state)
        .nest("/studio/assets", crate::assets::router());
    crate::access::protect_remote_router(router)
}

#[derive(Deserialize)]
pub(crate) struct ProjectQuery {
    project: Option<String>,
    notice: Option<String>,
}

#[derive(Deserialize)]
struct AddForm {
    form_token: String,
    name: String,
    base_url: String,
    token: String,
}

#[derive(Deserialize)]
struct RemoveForm {
    form_token: String,
    name: String,
}

/// Compares the submitted form token in constant time.
pub(crate) fn form_token_valid(expected: &str, submitted: &str) -> bool {
    let (expected, submitted) = (expected.as_bytes(), submitted.as_bytes());
    expected.len() == submitted.len()
        && expected
            .iter()
            .zip(submitted)
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
}

pub(crate) fn names(projects: &[Project]) -> Vec<String> {
    projects
        .iter()
        .map(|project| project.name.clone())
        .collect()
}

/// Renders a full page with the remote chrome.
pub(crate) fn render(
    state: &RemoteState,
    projects: &[String],
    selected: Option<&str>,
    active: Page,
    title: &str,
    content: &str,
) -> Html<String> {
    let _ = state;
    let chrome = Chrome {
        projects,
        selected,
        active,
    };
    Html(pages::layout(&chrome, title, content))
}

fn storage_error(state: &RemoteState, error: ProjectError) -> Response {
    let content = pages::notice(
        "error",
        "The project list is unavailable",
        &error.to_string(),
    );
    (
        StatusCode::SERVICE_UNAVAILABLE,
        render(state, &[], None, Page::Projects, "Projects", &content),
    )
        .into_response()
}

fn projects_response(
    state: &RemoteState,
    status: StatusCode,
    message: Option<(&str, &str, &str)>,
) -> Response {
    let projects = match state.store.load() {
        Ok(projects) => projects,
        Err(error) => return storage_error(state, error),
    };
    let rows: Vec<pages::projects::ProjectRow<'_>> = projects
        .iter()
        .map(|project| pages::projects::ProjectRow {
            project,
            status: state.monitoring.last_status(&project.name),
        })
        .collect();
    let content = pages::projects::page(
        &rows,
        &state.form_token,
        message,
        state.store.readable_by_others(),
    );
    (
        status,
        render(
            state,
            &names(&projects),
            None,
            Page::Projects,
            "Projects",
            &content,
        ),
    )
        .into_response()
}

async fn projects_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<ProjectQuery>,
) -> Response {
    let message = match query.notice.as_deref() {
        Some("added") => Some((
            "ok",
            "Project added",
            "Its token is encrypted on this machine.",
        )),
        Some("removed") => Some((
            "ok",
            "Project removed",
            "Its token was deleted from this machine.",
        )),
        _ => None,
    };
    projects_response(&state, StatusCode::OK, message)
}

fn refused(state: &RemoteState) -> Response {
    projects_response(
        state,
        StatusCode::FORBIDDEN,
        Some((
            "error",
            "The form was refused",
            "Reload the page and submit it again from this Studio.",
        )),
    )
}

async fn add_project(
    State(state): State<Arc<RemoteState>>,
    marker: Option<Extension<VerifiedLocalStudioAccess>>,
    Form(form): Form<AddForm>,
) -> Response {
    if marker.is_none() || !form_token_valid(&state.form_token, &form.form_token) {
        return refused(&state);
    }
    match state
        .store
        .add(form.name.trim(), &form.base_url, &form.token)
    {
        Ok(()) => Redirect::to("/studio?notice=added").into_response(),
        Err(error) => projects_response(
            &state,
            StatusCode::UNPROCESSABLE_ENTITY,
            Some(("error", "The project was not added", &error.to_string())),
        ),
    }
}

async fn remove_project(
    State(state): State<Arc<RemoteState>>,
    marker: Option<Extension<VerifiedLocalStudioAccess>>,
    Form(form): Form<RemoveForm>,
) -> Response {
    if marker.is_none() || !form_token_valid(&state.form_token, &form.form_token) {
        return refused(&state);
    }
    match state.store.remove(&form.name) {
        Ok(()) => {
            state.monitoring.forget(&form.name);
            Redirect::to("/studio?notice=removed").into_response()
        }
        Err(error) => projects_response(
            &state,
            StatusCode::UNPROCESSABLE_ENTITY,
            Some(("error", "The project was not removed", &error.to_string())),
        ),
    }
}

/// The selected project (the first when none is named), with every name.
pub(crate) fn select(
    state: &RemoteState,
    query: &ProjectQuery,
    active: Page,
) -> Result<(Project, Vec<String>), Box<Response>> {
    let projects = state
        .store
        .load()
        .map_err(|error| Box::new(storage_error(state, error)))?;
    let all = names(&projects);
    let wanted = query.project.as_deref();
    let selected = match wanted {
        Some(name) => projects.into_iter().find(|project| project.name == name),
        None => projects.into_iter().next(),
    };
    selected
        .map(|project| (project, all.clone()))
        .ok_or_else(|| {
            let (status, title, detail) = if wanted.is_some() {
                (
                    StatusCode::NOT_FOUND,
                    "Unknown project",
                    "No project has this name; pick one in the switcher.",
                )
            } else {
                (
                    StatusCode::OK,
                    "No projects yet",
                    "Add a project on the Projects page first.",
                )
            };
            let response = (
                status,
                render(
                    state,
                    &all,
                    None,
                    active,
                    title,
                    &pages::notice("warn", title, detail),
                ),
            )
                .into_response();
            Box::new(response)
        })
}

async fn monitor_page(
    state: Arc<RemoteState>,
    query: ProjectQuery,
    active: Page,
    title: &str,
    body: fn(&str, &super::document::MonitorDocument) -> String,
) -> Response {
    let (project, all) = match select(&state, &query, active) {
        Ok(selected) => selected,
        Err(response) => return *response,
    };
    let fetched = state.client.fetch(&project).await;
    state.monitoring.observe(&project.name, &fetched);
    let content = match &fetched {
        Ok(document) => body(&project.name, document),
        Err(error) => pages::notice(
            "error",
            &format!("{} is unavailable", project.name),
            &error.message(),
        ),
    };
    render(&state, &all, Some(&project.name), active, title, &content).into_response()
}

async fn overview_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<ProjectQuery>,
) -> Response {
    monitor_page(
        state,
        query,
        Page::Overview,
        "Overview",
        pages::monitor::overview,
    )
    .await
}

async fn routes_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<ProjectQuery>,
) -> Response {
    monitor_page(state, query, Page::Routes, "Routes", pages::monitor::routes).await
}

async fn queues_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<ProjectQuery>,
) -> Response {
    monitor_page(state, query, Page::Queues, "Queues", pages::monitor::queues).await
}

async fn queries_page(
    State(state): State<Arc<RemoteState>>,
    Query(query): Query<ProjectQuery>,
) -> Response {
    monitor_page(
        state,
        query,
        Page::Queries,
        "N+1 queries",
        pages::monitor::queries,
    )
    .await
}
