// Idempotent in-app notification templates for Academy domain events.

mod migration;
mod models;
#[path = "notification_controller.rs"]
mod notification_controller;
mod templates;
#[cfg(test)]
mod tests;

use migration::NOTIFICATION_MIGRATION;
use models::{NOTIFICATION_MODEL, NOTIFICATION_PREFERENCE_MODEL};
use templates::NOTIFICATION_TEMPLATE_SERVICE;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
        ("src/models/notification.rs", NOTIFICATION_MODEL.to_string()),
        (
            "src/models/notification_preference.rs",
            NOTIFICATION_PREFERENCE_MODEL.to_string(),
        ),
        (
            "src/services/notification_service.rs",
            NOTIFICATION_SERVICE.to_string(),
        ),
        (
            "src/services/notification_template_service.rs",
            NOTIFICATION_TEMPLATE_SERVICE.to_string(),
        ),
        (
            "src/migrations/m20260830000000_add_notifications.rs",
            NOTIFICATION_MIGRATION.to_string(),
        ),
    ];
    files.extend(notification_controller::get_files());
    files
}

const NOTIFICATION_SERVICE: &str = r##"use crate::services::notification_template_service::{
    NotificationTemplateError, render_notification,
};
use crate::services::school_service;
use rullst::{BroadcastManager, TenantRealtime};
use rullst::security::TenantContext;
use rullst_security::{RbacGuard, UserContext};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationReceipt {
    pub notification_key: String,
    pub applied: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotificationView {
    pub id: i32,
    pub channel: String,
    pub locale: String,
    pub localization_key: String,
    pub payload_json: String,
    pub rendered_locale: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreferenceReceipt {
    pub channel: String,
    pub enabled: bool,
    pub locale: String,
    pub applied: bool,
}

#[derive(Debug)]
pub enum NotificationError {
    InvalidField(&'static str),
    ClaimNotHeld,
    Forbidden,
    Realtime(String),
    Template(NotificationTemplateError),
    InvalidJson(serde_json::Error),
    Database(rullst_orm::Error),
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(formatter, "invalid notification field: {field}"),
            Self::ClaimNotHeld => formatter.write_str("notification source event is not held by this claim"),
            Self::Forbidden => formatter.write_str("notification access denied"),
            Self::Realtime(error) => write!(formatter, "notification realtime error: {error}"),
            Self::Template(error) => write!(formatter, "notification template error: {error}"),
            Self::InvalidJson(error) => write!(formatter, "invalid notification JSON: {error}"),
            Self::Database(error) => write!(formatter, "notification database error: {error}"),
        }
    }
}

impl std::error::Error for NotificationError {}

impl From<rullst_orm::Error> for NotificationError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AchievementAwardedV1 {
    schema_version: i32,
    actor_user_id: i32,
    subject_user_id: i32,
    achievement_code: String,
    execution_key: String,
}

fn authorize_subject(context: &UserContext, subject_user_id: i32) -> Result<(), NotificationError> {
    RbacGuard::authorize_owner_or_role(context, &subject_user_id.to_string(), "admin")
        .map_err(|_| NotificationError::Forbidden)
}

fn realtime_manager() -> Arc<BroadcastManager> {
    static MANAGER: OnceLock<Arc<BroadcastManager>> = OnceLock::new();
    Arc::clone(MANAGER.get_or_init(|| Arc::new(BroadcastManager::new())))
}

fn tenant_realtime(tenant_key: &str) -> Result<TenantRealtime, NotificationError> {
    let tenant_context = TenantContext::try_new(tenant_key)
        .map_err(|error| NotificationError::Realtime(error.to_string()))?;
    Ok(TenantRealtime::from_context(realtime_manager(), &tenant_context))
}

pub async fn subscribe_in_app(
    context: &UserContext,
    subject_user_id: i32,
) -> Result<tokio::sync::broadcast::Receiver<rullst::RealtimeMessage>, NotificationError> {
    if subject_user_id <= 0 { return Err(NotificationError::InvalidField("subscription")); }
    authorize_subject(context, subject_user_id)?;
    school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => NotificationError::Database(error),
            _ => NotificationError::Forbidden,
        })?;
    let tenant_key = context.tenant_id().ok_or(NotificationError::Forbidden)?;
    tenant_realtime(tenant_key)?
        .subscribe(&format!("notifications/user/{subject_user_id}"))
        .map_err(|error| NotificationError::Realtime(error.to_string()))
}

pub async fn list_notifications(
    context: &UserContext,
    subject_user_id: i32,
    status: Option<&str>,
    before_id: i32,
    limit: i64,
) -> Result<Vec<NotificationView>, NotificationError> {
    if subject_user_id <= 0 || before_id < 0 || !(1..=100).contains(&limit) {
        return Err(NotificationError::InvalidField("notification query"));
    }
    authorize_subject(context, subject_user_id)?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => NotificationError::Database(error),
            _ => NotificationError::Forbidden,
        })?;
    let status = status.unwrap_or("");
    if !matches!(status, "" | "unread" | "read" | "suppressed") {
        return Err(NotificationError::InvalidField("status"));
    }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT id, channel, locale, localization_key, payload_json, status, created_at FROM notifications WHERE school_id = $1 AND user_id = $2 AND ($3 = '' OR status = $3) AND ($4 = 0 OR id < $4) ORDER BY id DESC LIMIT $5",
        _ => "SELECT id, channel, locale, localization_key, payload_json, status, created_at FROM notifications WHERE school_id = ? AND user_id = ? AND (? = '' OR status = ?) AND (? = 0 OR id < ?) ORDER BY id DESC LIMIT ?",
    };
    let mut query = rullst::db::sqlx::query_as::<_, (i32, String, String, String, String, String, String)>(sql)
        .bind(school_id)
        .bind(subject_user_id)
        .bind(status);
    if driver != "postgres" {
        query = query.bind(status);
    }
    query = query.bind(before_id);
    if driver != "postgres" {
        query = query.bind(before_id);
    }
    let rows = query
        .bind(limit)
        .fetch_all(rullst::db::Orm::pool()?)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;
    rows.into_iter()
        .map(|row| {
            let rendered = render_notification(&row.2, &row.3, &row.4)
                .map_err(NotificationError::Template)?;
            Ok(NotificationView {
                    id: row.0,
                    channel: row.1,
                    locale: row.2,
                    localization_key: row.3,
                    payload_json: row.4,
                    rendered_locale: rendered.locale,
                    title: rendered.title,
                    body: rendered.body,
                    status: row.5,
                    created_at: row.6,
                })
        })
        .collect()
}

pub async fn set_preference(
    context: &UserContext,
    subject_user_id: i32,
    channel: &str,
    enabled: bool,
    locale: &str,
) -> Result<PreferenceReceipt, NotificationError> {
    if subject_user_id <= 0
        || !matches!(channel, "in_app" | "email" | "push" | "realtime")
        || !valid_locale(locale)
    {
        return Err(NotificationError::InvalidField("preference"));
    }
    authorize_subject(context, subject_user_id)?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => NotificationError::Database(error),
            _ => NotificationError::Forbidden,
        })?;
    let enabled_i32 = i32::from(enabled);
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;
    let existing_sql = match driver {
        "postgres" => "SELECT enabled, locale FROM notification_preferences WHERE school_id = $1 AND user_id = $2 AND channel = $3",
        _ => "SELECT enabled, locale FROM notification_preferences WHERE school_id = ? AND user_id = ? AND channel = ?",
    };
    let existing = rullst::db::sqlx::query_as::<_, (i32, String)>(existing_sql)
        .bind(school_id)
        .bind(subject_user_id)
        .bind(channel)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;
    if existing.as_ref().is_some_and(|value| value.0 == enabled_i32 && value.1 == locale) {
        return Ok(PreferenceReceipt {
            channel: channel.to_string(),
            enabled,
            locale: locale.to_string(),
            applied: false,
        });
    }
    let upsert_sql = match driver {
        "postgres" => "INSERT INTO notification_preferences (school_id, user_id, channel, enabled, locale, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (school_id, user_id, channel) DO UPDATE SET enabled = EXCLUDED.enabled, locale = EXCLUDED.locale, updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO notification_preferences (school_id, user_id, channel, enabled, locale, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE enabled = VALUES(enabled), locale = VALUES(locale), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO notification_preferences (school_id, user_id, channel, enabled, locale, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (school_id, user_id, channel) DO UPDATE SET enabled = excluded.enabled, locale = excluded.locale, updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(upsert_sql)
        .bind(school_id)
        .bind(subject_user_id)
        .bind(channel)
        .bind(enabled_i32)
        .bind(locale)
        .execute(&mut *transaction)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;
    transaction
        .commit()
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;
    Ok(PreferenceReceipt {
        channel: channel.to_string(),
        enabled,
        locale: locale.to_string(),
        applied: true,
    })
}

pub async fn deliver_claimed_achievement(
    event_key: &str,
    claim_key: &str,
) -> Result<NotificationReceipt, NotificationError> {
    if !valid_key(event_key, 256) || !valid_key(claim_key, 128) {
        return Err(NotificationError::InvalidField("claim"));
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;
    let event_sql = match driver {
        "postgres" => "SELECT ao.school_id, ao.subject_user_id, ao.payload_json, s.tenant_key FROM academy_outbox ao INNER JOIN schools s ON s.id = ao.school_id AND s.status = 'active' WHERE ao.event_key = $1 AND ao.event_kind = $2 AND ao.status = $3 AND ao.claim_key = $4",
        _ => "SELECT ao.school_id, ao.subject_user_id, ao.payload_json, s.tenant_key FROM academy_outbox ao INNER JOIN schools s ON s.id = ao.school_id AND s.status = 'active' WHERE ao.event_key = ? AND ao.event_kind = ? AND ao.status = ? AND ao.claim_key = ?",
    };
    let event = rullst::db::sqlx::query_as::<_, (i32, i32, String, String)>(event_sql)
        .bind(event_key)
        .bind("achievement_awarded")
        .bind("processing")
        .bind(claim_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?
        .ok_or(NotificationError::ClaimNotHeld)?;
    let payload: AchievementAwardedV1 =
        serde_json::from_str(&event.2).map_err(NotificationError::InvalidJson)?;
    if payload.schema_version != 1
        || event.0 <= 0
        || payload.actor_user_id <= 0
        || payload.subject_user_id <= 0
        || payload.subject_user_id != event.1
        || !valid_key(&payload.achievement_code, 64)
        || !valid_key(&payload.execution_key, 256)
    {
        return Err(NotificationError::InvalidField("achievement event"));
    }
    let realtime = tenant_realtime(&event.3)?;
    let preference_sql = match driver {
        "postgres" => "SELECT enabled, locale FROM notification_preferences WHERE school_id = $1 AND user_id = $2 AND channel = $3",
        _ => "SELECT enabled, locale FROM notification_preferences WHERE school_id = ? AND user_id = ? AND channel = ?",
    };
    let preference = rullst::db::sqlx::query_as::<_, (i32, String)>(preference_sql)
        .bind(event.0)
        .bind(payload.subject_user_id)
        .bind("in_app")
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?;
    let (enabled, locale) = preference.unwrap_or((1, "en".to_string()));
    if !matches!(enabled, 0 | 1) || !valid_locale(&locale) {
        return Err(NotificationError::InvalidField("preference"));
    }
    let realtime_preference =
        rullst::db::sqlx::query_as::<_, (i32, String)>(preference_sql)
            .bind(event.0)
            .bind(payload.subject_user_id)
            .bind("realtime")
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| NotificationError::Database(error.into()))?;
    let (realtime_enabled, realtime_locale) =
        realtime_preference.unwrap_or((1, locale.clone()));
    if !matches!(realtime_enabled, 0 | 1) || !valid_locale(&realtime_locale) {
        return Err(NotificationError::InvalidField("realtime preference"));
    }
    let status = if enabled == 1 { "unread" } else { "suppressed" };
    let notification_key = format!("notification:{event_key}:in_app");
    let notification_body = serde_json::json!({
        "schema_version": 1,
        "achievement_code": payload.achievement_code,
        "recorded_actor_user_id": payload.actor_user_id,
    });
    let notification_payload = notification_body.to_string();
    let realtime_rendered = render_notification(
        &realtime_locale,
        "academy.achievement.awarded",
        &notification_payload,
    ).map_err(NotificationError::Template)?;
    let realtime_payload = serde_json::json!({
        "schema_version": 1,
        "notification_key": notification_key,
        "subject_user_id": payload.subject_user_id,
        "channel": "in_app",
        "locale": realtime_locale,
        "localization_key": "academy.achievement.awarded",
        "payload": notification_body,
        "rendered": realtime_rendered,
    }).to_string();
    let insert_sql = match driver {
        "postgres" => "INSERT INTO notifications (school_id, notification_key, user_id, channel, locale, localization_key, payload_json, status, source_event_key, read_at, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO notifications (school_id, notification_key, user_id, channel, locale, localization_key, payload_json, status, source_event_key, read_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO notifications (school_id, notification_key, user_id, channel, locale, localization_key, payload_json, status, source_event_key, read_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    let applied = rullst::db::sqlx::query(insert_sql)
        .bind(event.0)
        .bind(&notification_key)
        .bind(payload.subject_user_id)
        .bind("in_app")
        .bind(&locale)
        .bind("academy.achievement.awarded")
        .bind(&notification_payload)
        .bind(status)
        .bind(event_key)
        .bind("")
        .execute(&mut *transaction)
        .await
        .map_err(|error| NotificationError::Database(error.into()))?
        .rows_affected() == 1;
    transaction.commit().await.map_err(|error| NotificationError::Database(error.into()))?;
    if applied && status == "unread" && realtime_enabled == 1 {
        let _ = realtime.publish(
            &format!("notifications/user/{}", payload.subject_user_id),
            "notification.created",
            &realtime_payload,
        );
    }
    Ok(NotificationReceipt {
        notification_key,
        applied,
        status: status.to_string(),
    })
}

pub async fn mark_read(
    context: &UserContext,
    subject_user_id: i32,
    notification_id: i32,
) -> Result<bool, NotificationError> {
    if subject_user_id <= 0 || notification_id <= 0 {
        return Err(NotificationError::InvalidField("read request"));
    }
    authorize_subject(context, subject_user_id)?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => NotificationError::Database(error),
            _ => NotificationError::Forbidden,
        })?;
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "UPDATE notifications SET status = $1, read_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND school_id = $3 AND user_id = $4 AND status = $5",
        _ => "UPDATE notifications SET status = ?, read_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND school_id = ? AND user_id = ? AND status = ?",
    };
    rullst::db::sqlx::query(sql)
        .bind("read")
        .bind(notification_id)
        .bind(school_id)
        .bind(subject_user_id)
        .bind("unread")
        .execute(rullst::db::Orm::pool()?)
        .await
        .map(|result| result.rows_affected() == 1)
        .map_err(|error| NotificationError::Database(error.into()))
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}

fn valid_locale(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 16
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic() || byte == b'-')
}
"##;
