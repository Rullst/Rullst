//! Application-owned resource for the Linux/SQLite SaaS journey, not a blueprint default.
use rullst::db::{Orm, sqlx};
use rullst::security::{TenantContext, TenantMembership};
use rullst::server::{Extension, Json, Next, Path, Request, Response, StatusCode, from_fn};
use rullst::{Router, TenantConfig, TenantLayer, TenantStrategy};
use rullst_security::{RbacGuard, UserContext};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router {
    Router::new()
        .route("/journey/notes", rullst::routing::post(create))
        .route(
            "/journey/notes/{id}",
            rullst::routing::get(read).put(update),
        )
        .layer(TenantLayer::new(TenantConfig::new(TenantStrategy::Header)))
        .layer(from_fn(bind_membership))
        .layer(from_fn(
            crate::middlewares::auth_middleware::auth_middleware,
        ))
        // Explicitly protect the additional application resource even in the
        // loopback development profile; Server's production baseline is unchanged.
        .layer(from_fn(rullst::security::csrf_middleware))
        .layer(from_fn(rullst::security::headers_middleware))
        .layer(from_fn(rullst::security::waf_middleware))
}

// The generated Auth middleware has already loaded this user from its database.
// Membership is provisioned by the fixture operator, never by a browser claim.
async fn bind_membership(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let user_id = *req
        .extensions()
        .get::<i32>()
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let pool = Orm::pool().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let tenants: Vec<String> =
        sqlx::query_scalar("SELECT tenant_id FROM journey_memberships WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let membership = TenantMembership::try_new(tenants).map_err(|_| StatusCode::FORBIDDEN)?;
    req.extensions_mut().insert(membership);
    Ok(next.run(req).await)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    body: String,
}

impl Input {
    fn validate(&self) -> Result<(), StatusCode> {
        if self.body.is_empty() || self.body.len() > 256 {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Ok(())
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct Note {
    id: i64,
    tenant_id: String,
    owner_id: i32,
    body: String,
}

async fn authorized_note(
    id: i64,
    user_id: i32,
    tenant: &TenantContext,
) -> Result<Note, StatusCode> {
    let pool = Orm::pool().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let note: Note = sqlx::query_as(
        "SELECT id, tenant_id, owner_id, body FROM journey_notes WHERE id = ? AND tenant_id = ?",
    )
    .bind(id)
    .bind(&tenant.tenant_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let context = UserContext::new(user_id.to_string(), vec![])
        .try_with_tenant_id(&tenant.tenant_id)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    RbacGuard::authorize_tenant_owner_or_role(
        &context,
        &note.tenant_id,
        &note.owner_id.to_string(),
        "journey-admin",
    )
    .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok(note)
}

async fn create(
    Extension(user_id): Extension<i32>,
    Extension(tenant): Extension<TenantContext>,
    Json(input): Json<Input>,
) -> Result<(StatusCode, Json<Note>), StatusCode> {
    input.validate()?;
    let pool = Orm::pool().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // Returning the inserted row keeps its identity tied to the same SQL statement.
    let note = sqlx::query_as(
        "INSERT INTO journey_notes (tenant_id, owner_id, body) VALUES (?, ?, ?) \
         RETURNING id, tenant_id, owner_id, body",
    )
    .bind(&tenant.tenant_id)
    .bind(user_id)
    .bind(input.body)
    .fetch_one(pool)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok((StatusCode::CREATED, Json(note)))
}

async fn read(
    Extension(user_id): Extension<i32>,
    Extension(tenant): Extension<TenantContext>,
    Path(id): Path<i64>,
) -> Result<Json<Note>, StatusCode> {
    authorized_note(id, user_id, &tenant).await.map(Json)
}

async fn update(
    Extension(user_id): Extension<i32>,
    Extension(tenant): Extension<TenantContext>,
    Path(id): Path<i64>,
    Json(input): Json<Input>,
) -> Result<Json<Note>, StatusCode> {
    input.validate()?;
    let mut note = authorized_note(id, user_id, &tenant).await?;
    let pool = Orm::pool().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let result = sqlx::query(
        "UPDATE journey_notes SET body = ? WHERE id = ? AND tenant_id = ? AND owner_id = ?",
    )
    .bind(&input.body)
    .bind(id)
    .bind(&tenant.tenant_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if result.rows_affected() != 1 {
        return Err(StatusCode::NOT_FOUND);
    }
    note.body = input.body;
    Ok(Json(note))
}
