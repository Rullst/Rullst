pub(super) const AUTH_MIDDLEWARE: &str = r##"use crate::models::user::User;
use crate::services::{role_service, school_service};
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
            let observed_at_epoch = match std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
            {
                Some(value) => value,
                None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            let requested_school = match request.headers().get("x-school-id") {
                Some(value) => match value.to_str() {
                    Ok(value) => Some(value),
                    Err(_) => return StatusCode::FORBIDDEN.into_response(),
                },
                None => None,
            };
            let resolved_school = match school_service::resolve_membership_at(
                user_id,
                requested_school,
                observed_at_epoch,
            )
            .await
            {
                Ok(school) => school,
                Err(error) => {
                    eprintln!("Authentication school membership denied: {error}");
                    return StatusCode::FORBIDDEN.into_response();
                }
            };
            let seed_context = match UserContext::new(
                user_id.to_string(),
                vec!["student".to_string()],
            )
            .try_with_tenant_id(resolved_school.tenant_key.clone())
            {
                Ok(context) => context,
                Err(error) => {
                    eprintln!("Authentication tenant context invalid: {error}");
                    return StatusCode::FORBIDDEN.into_response();
                }
            };
            let roles = match role_service::active_roles_at(
                &seed_context,
                user_id,
                observed_at_epoch,
            )
            .await
            {
                Ok(roles) => roles,
                Err(error) => {
                    eprintln!("Authentication role query failed: {error}");
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
            };
            let context = match UserContext::new(user_id.to_string(), roles)
                .try_with_tenant_id(resolved_school.tenant_key.clone())
            {
                Ok(context) => context,
                Err(error) => {
                    eprintln!("Authentication tenant context invalid: {error}");
                    return StatusCode::FORBIDDEN.into_response();
                }
            };
            let tenant_context = match rullst::security::TenantContext::try_new(
                resolved_school.tenant_key,
            ) {
                Ok(context) => context,
                Err(error) => {
                    eprintln!("Authentication tenant extension invalid: {error}");
                    return StatusCode::FORBIDDEN.into_response();
                }
            };
            request.extensions_mut().insert(user_id);
            request.extensions_mut().insert(context);
            request.extensions_mut().insert(tenant_context);
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
