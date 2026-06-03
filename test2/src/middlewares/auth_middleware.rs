use rullst::server::{
    Request,
    Next,
    Response, Redirect, IntoResponse,
};

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let headers = req.headers();
    if let Some(cookie) = rullst::auth::extract_session_cookie(headers) {
        let app_key = rullst::auth::get_app_key();
        if let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) {
            req.extensions_mut().insert(user_id);
            return next.run(req).await;
        }
    }
    Redirect::to("/login").into_response()
}
