use crate::models::user::User;
use rullst::server::{IntoResponse, Next, Redirect, Request, Response, StatusCode};
use rullst_security::UserContext;

/// Resolves the signed-in learner from the `rullst_session` cookie. A cookie
/// counts only while it carries the account's current `session_epoch`, so
/// `User::revoke_sessions` signs out every device.
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
    let Ok(claims) = rullst::auth::decrypt_session_claims(&cookie, &app_key) else {
        return Redirect::to("/login").into_response();
    };
    match User::find(claims.user_id).await {
        Ok(Some(user)) if user.accepts_session(&claims) => {
            request.extensions_mut().insert(user.id);
            request.extensions_mut().insert(UserContext::new(
                user.id.to_string(),
                vec!["student".to_string()],
            ));
            next.run(request).await
        }
        // Unknown account or revoked cookie.
        Ok(_) => Redirect::to("/login").into_response(),
        Err(error) => {
            eprintln!("Authentication user query failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
