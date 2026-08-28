pub(super) const FOUNDATION_AUTH_MIDDLEWARE: &str = r##"use crate::models::user::User;
use rullst::server::{IntoResponse, Next, Redirect, Request, Response, StatusCode};
use rullst_security::UserContext;

pub async fn auth_middleware(mut request: Request, next: Next) -> Response {
    let Some(cookie) = rullst::auth::extract_session_cookie(request.headers()) else {
        return Redirect::to("/login").into_response();
    };
    let app_key = match rullst::auth::get_app_key() {
        Ok(key) => key,
        Err(error) => {
            eprintln!("Authentication key unavailable: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) else {
        return Redirect::to("/login").into_response();
    };
    match User::find(user_id).await {
        Ok(Some(_)) => {
            request.extensions_mut().insert(user_id);
            request.extensions_mut().insert(UserContext::new(
                user_id.to_string(),
                vec!["student".to_string()],
            ));
            next.run(request).await
        }
        Ok(None) => Redirect::to("/login").into_response(),
        Err(error) => {
            eprintln!("Authentication user query failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
"##;
