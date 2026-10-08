use rullst::html::RawHtml;
use rullst::security::{CspNonce, CsrfToken};
use rullst::server::{Extension, Html, IntoResponse, Path, Redirect, Response, StatusCode};
use rullst::{html, HtmxRequest, HtmxResponse, Validate, ValidatedForm};
use rullst_security::{RbacGuard, UserContext};
use serde::Deserialize;

use crate::models::note::Note;

/// The fields a user may submit. `user_id` is not here: it comes from the session.
#[derive(Debug, Deserialize, Validate)]
pub struct NoteForm {
    #[validate(length(min = 1, max = 120, message = "Give the note a title (1 to 120 characters)."))]
    pub title: String,
    #[validate(length(max = 5000, message = "Keep the note under 5000 characters."))]
    pub body: String,
}

/// Lets only the note's owner through. Another user's note answers 404, so
/// its existence is not revealed.
pub fn authorize(user_id: i32, note: &Note) -> Result<(), StatusCode> {
    let user = UserContext::new(user_id.to_string(), Vec::new());
    RbacGuard::authorize_owner_or_role(&user, &note.user_id.to_string(), "admin")
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn find_owned(user_id: i32, id: i32) -> Result<Note, StatusCode> {
    let note = Note::find(id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::NOT_FOUND)?;
    authorize(user_id, &note)?;
    Ok(note)
}

/// After a successful write, HTMX gets an `HX-Redirect`; a plain form post a 303.
fn back_to_list(htmx: &HtmxRequest) -> Response {
    if htmx.is_htmx {
        HtmxResponse::new("").redirect("/notes").into_response()
    } else {
        Redirect::to("/notes").into_response()
    }
}

fn page(nonce: &CspNonce, title: &str, content: String) -> Html<String> {
    Html(format!("<!DOCTYPE html>{}", html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>{title}</title>
                <link rel="stylesheet" href="/static/rullst.css" />
                <script nonce={nonce.as_str()} src="/static/htmx-1.9.12.min.js"></script>
            </head>
            <body><main class="container">{RawHtml(content)}</main></body>
        </html>
    }))
}

fn note_form(action: &str, csrf: &CsrfToken, title: &str, body: &str, button: &str) -> String {
    html! {
        <form method="post" action={action} hx-post={action} hx-target="#errors">
            <input type="hidden" name="_token" value={csrf.as_str()} />
            <label for="title">"Title"</label>
            <input id="title" name="title" value={title} required="true" />
            <label for="body">"Note"</label>
            <textarea id="body" name="body">{body}</textarea>
            <div id="errors" role="alert"></div>
            <button type="submit">{button}</button>
        </form>
    }
}

/// GET /notes: the signed-in user's notes and a form to add one.
pub async fn index(
    Extension(user_id): Extension<i32>,
    Extension(csrf): Extension<CsrfToken>,
    Extension(nonce): Extension<CspNonce>,
) -> Result<Html<String>, StatusCode> {
    let notes = Note::query()
        .where_eq("user_id", user_id)
        .order_by_desc("id")
        .get()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let items: String = notes
        .iter()
        .map(|note| {
            let href = format!("/notes/{}", note.id);
            html! { <li><a href={href}>{note.title.as_str()}</a></li> }
        })
        .collect();
    let content = html! {
        <h1>"My notes"</h1>
        <ul id="notes">{RawHtml(items)}</ul>
        <h2>"New note"</h2>
        {RawHtml(note_form("/notes", &csrf, "", "", "Add note"))}
    };
    Ok(page(&nonce, "My notes", content))
}

/// POST /notes: validates the form, then saves a note owned by the current user.
pub async fn store(
    htmx: HtmxRequest,
    Extension(user_id): Extension<i32>,
    ValidatedForm(form): ValidatedForm<NoteForm>,
) -> Result<Response, StatusCode> {
    let mut note = Note { id: 0, user_id, title: form.title, body: form.body };
    note.save().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(back_to_list(&htmx))
}

/// GET /notes/{id}: one note, with edit and delete forms. Owner only.
pub async fn show(
    Path(id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(csrf): Extension<CsrfToken>,
    Extension(nonce): Extension<CspNonce>,
) -> Result<Html<String>, StatusCode> {
    let note = find_owned(user_id, id).await?;
    let action = format!("/notes/{id}");
    let delete = format!("/notes/{id}/delete");
    let content = html! {
        <p><a href="/notes">"← All notes"</a></p>
        <h1>{note.title.as_str()}</h1>
        {RawHtml(note_form(&action, &csrf, &note.title, &note.body, "Save"))}
        <form method="post" action={delete.as_str()}>
            <input type="hidden" name="_token" value={csrf.as_str()} />
            <button type="submit">"Delete"</button>
        </form>
    };
    Ok(page(&nonce, &note.title, content))
}

/// POST /notes/{id}: validates and saves changes. Owner only.
pub async fn update(
    htmx: HtmxRequest,
    Path(id): Path<i32>,
    Extension(user_id): Extension<i32>,
    ValidatedForm(form): ValidatedForm<NoteForm>,
) -> Result<Response, StatusCode> {
    let mut note = find_owned(user_id, id).await?;
    note.title = form.title;
    note.body = form.body;
    note.save().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(back_to_list(&htmx))
}

/// POST /notes/{id}/delete: deletes the note. Owner only.
pub async fn destroy(
    htmx: HtmxRequest,
    Path(id): Path<i32>,
    Extension(user_id): Extension<i32>,
) -> Result<Response, StatusCode> {
    let note = find_owned(user_id, id).await?;
    note.delete().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(back_to_list(&htmx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_of(user_id: i32) -> Note {
        Note { id: 1, user_id, title: "Groceries".into(), body: String::new() }
    }

    #[test]
    fn only_the_owner_may_open_a_note() {
        assert_eq!(authorize(7, &note_of(7)), Ok(()));
        assert_eq!(authorize(8, &note_of(7)), Err(StatusCode::NOT_FOUND));
    }

    #[test]
    fn an_empty_title_is_rejected() {
        let form = NoteForm { title: String::new(), body: "text".into() };
        assert!(form.validate().is_err());
        let form = NoteForm { title: "Groceries".into(), body: String::new() };
        assert!(form.validate().is_ok());
    }
}
