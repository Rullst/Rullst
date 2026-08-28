pub(super) const SCHOOL_SERVICE: &str = r##"use rullst_security::{RbacGuard, UserContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSchool {
    pub school_id: i32,
    pub tenant_key: String,
}

#[derive(Debug)]
pub enum SchoolError {
    Forbidden,
    AmbiguousMembership,
    InvalidField(&'static str),
    Database(rullst_orm::Error),
}

impl std::fmt::Display for SchoolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("school-scoped access denied"),
            Self::AmbiguousMembership => formatter.write_str("an explicit active school selection is required"),
            Self::InvalidField(field) => write!(formatter, "invalid school field: {field}"),
            Self::Database(error) => write!(formatter, "school database error: {error}"),
        }
    }
}

impl std::error::Error for SchoolError {}

impl From<rullst_orm::Error> for SchoolError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn valid_tenant_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

fn actor_id(context: &UserContext) -> Result<i32, SchoolError> {
    context.user_id.parse::<i32>().map_err(|_| SchoolError::Forbidden)
}

pub async fn resolve_membership_at(
    user_id: i32,
    requested_tenant_key: Option<&str>,
    observed_at_epoch: i64,
) -> Result<ResolvedSchool, SchoolError> {
    if user_id <= 0 || observed_at_epoch <= 0 {
        return Err(SchoolError::InvalidField("membership query"));
    }
    if requested_tenant_key.is_some_and(|value| !valid_tenant_key(value)) {
        return Err(SchoolError::Forbidden);
    }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT sm.school_id, s.tenant_key, sm.is_default FROM school_memberships sm INNER JOIN schools s ON s.id = sm.school_id WHERE sm.user_id = $1 AND sm.status = $2 AND s.status = $3 AND sm.valid_from_epoch <= $4 AND (sm.expires_at_epoch = 0 OR sm.expires_at_epoch > $5) ORDER BY sm.school_id ASC",
        _ => "SELECT sm.school_id, s.tenant_key, sm.is_default FROM school_memberships sm INNER JOIN schools s ON s.id = sm.school_id WHERE sm.user_id = ? AND sm.status = ? AND s.status = ? AND sm.valid_from_epoch <= ? AND (sm.expires_at_epoch = 0 OR sm.expires_at_epoch > ?) ORDER BY sm.school_id ASC",
    };
    let memberships = rullst::db::sqlx::query_as::<_, (i32, String, i32)>(sql)
        .bind(user_id).bind("active").bind("active")
        .bind(observed_at_epoch).bind(observed_at_epoch)
        .fetch_all(rullst::db::Orm::pool()?).await
        .map_err(|error| SchoolError::Database(error.into()))?;
    if memberships.is_empty() || memberships.len() > 64 {
        return Err(SchoolError::Forbidden);
    }
    if memberships.iter().any(|row| {
        row.0 <= 0 || !valid_tenant_key(&row.1) || !matches!(row.2, 0 | 1)
    }) {
        return Err(SchoolError::Forbidden);
    }

    let selected = if let Some(requested) = requested_tenant_key {
        memberships.iter().find(|row| row.1 == requested)
            .ok_or(SchoolError::Forbidden)?
    } else {
        let mut defaults = memberships.iter().filter(|row| row.2 == 1);
        let first = defaults.next();
        if defaults.next().is_some() {
            return Err(SchoolError::AmbiguousMembership);
        }
        match first {
            Some(row) => row,
            None if memberships.len() == 1 => memberships.first().ok_or(SchoolError::Forbidden)?,
            None => return Err(SchoolError::AmbiguousMembership),
        }
    };
    Ok(ResolvedSchool { school_id: selected.0, tenant_key: selected.1.clone() })
}

pub async fn context_school_id(context: &UserContext) -> Result<i32, SchoolError> {
    let tenant_key = context.tenant_id().ok_or(SchoolError::Forbidden)?;
    if !valid_tenant_key(tenant_key) { return Err(SchoolError::Forbidden); }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT id FROM schools WHERE tenant_key = $1 AND status = $2",
        _ => "SELECT id FROM schools WHERE tenant_key = ? AND status = ?",
    };
    rullst::db::sqlx::query_scalar::<_, i32>(sql).bind(tenant_key).bind("active")
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| SchoolError::Database(error.into()))?
        .filter(|school_id| *school_id > 0)
        .ok_or(SchoolError::Forbidden)
}

async fn authorize_membership_at(
    context: &UserContext,
    user_id: i32,
    school_id: i32,
    observed_at_epoch: i64,
) -> Result<(), SchoolError> {
    RbacGuard::authorize_tenant(
        context,
        context.tenant_id().ok_or(SchoolError::Forbidden)?,
    ).map_err(|_| SchoolError::Forbidden)?;
    let actor = actor_id(context)?;
    if actor != user_id && !context.has_role("admin") && !context.has_role("school_owner") {
        return Err(SchoolError::Forbidden);
    }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM school_memberships WHERE school_id = $1 AND user_id = $2 AND status = $3 AND valid_from_epoch <= $4 AND (expires_at_epoch = 0 OR expires_at_epoch > $5)",
        _ => "SELECT COUNT(*) FROM school_memberships WHERE school_id = ? AND user_id = ? AND status = ? AND valid_from_epoch <= ? AND (expires_at_epoch = 0 OR expires_at_epoch > ?)",
    };
    let count = rullst::db::sqlx::query_scalar::<_, i64>(sql)
        .bind(school_id).bind(user_id).bind("active")
        .bind(observed_at_epoch).bind(observed_at_epoch)
        .fetch_one(rullst::db::Orm::pool()?).await
        .map_err(|error| SchoolError::Database(error.into()))?;
    if count == 1 { Ok(()) } else { Err(SchoolError::Forbidden) }
}

pub async fn authorize_school_membership_at(
    context: &UserContext,
    user_id: i32,
    observed_at_epoch: i64,
) -> Result<i32, SchoolError> {
    let school_id = context_school_id(context).await?;
    authorize_membership_at(context, user_id, school_id, observed_at_epoch).await?;
    Ok(school_id)
}

pub async fn authorize_course(
    context: &UserContext,
    course_id: i32,
) -> Result<(i32, String), SchoolError> {
    if course_id <= 0 { return Err(SchoolError::InvalidField("course")); }
    let tenant_key = context.tenant_id().ok_or(SchoolError::Forbidden)?;
    RbacGuard::authorize_tenant(context, tenant_key).map_err(|_| SchoolError::Forbidden)?;
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT css.school_id, css.enrollment_policy FROM course_school_scopes css INNER JOIN schools s ON s.id = css.school_id WHERE css.course_id = $1 AND s.tenant_key = $2 AND s.status = $3",
        _ => "SELECT css.school_id, css.enrollment_policy FROM course_school_scopes css INNER JOIN schools s ON s.id = css.school_id WHERE css.course_id = ? AND s.tenant_key = ? AND s.status = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, String)>(sql)
        .bind(course_id).bind(tenant_key).bind("active")
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| SchoolError::Database(error.into()))?
        .ok_or(SchoolError::Forbidden)?;
    if row.0 <= 0 || !matches!(row.1.as_str(), "open" | "entitled") {
        return Err(SchoolError::Forbidden);
    }
    Ok(row)
}

pub async fn authorize_course_enrollment_at(
    context: &UserContext,
    user_id: i32,
    course_id: i32,
    observed_at_epoch: i64,
) -> Result<i32, SchoolError> {
    if observed_at_epoch <= 0 { return Err(SchoolError::InvalidField("clock")); }
    let actor = actor_id(context)?;
    if actor != user_id && !context.has_role("admin") { return Err(SchoolError::Forbidden); }
    let (school_id, enrollment_policy) = authorize_course(context, course_id).await?;
    authorize_membership_at(context, user_id, school_id, observed_at_epoch).await?;
    if enrollment_policy == "open" { return Ok(school_id); }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM course_entitlements WHERE school_id = $1 AND user_id = $2 AND course_id = $3 AND status = $4 AND starts_at_epoch <= $5 AND (expires_at_epoch = 0 OR expires_at_epoch > $6)",
        _ => "SELECT COUNT(*) FROM course_entitlements WHERE school_id = ? AND user_id = ? AND course_id = ? AND status = ? AND starts_at_epoch <= ? AND (expires_at_epoch = 0 OR expires_at_epoch > ?)",
    };
    let count = rullst::db::sqlx::query_scalar::<_, i64>(sql)
        .bind(school_id).bind(user_id).bind(course_id).bind("active")
        .bind(observed_at_epoch).bind(observed_at_epoch)
        .fetch_one(rullst::db::Orm::pool()?).await
        .map_err(|error| SchoolError::Database(error.into()))?;
    if count == 1 { Ok(school_id) } else { Err(SchoolError::Forbidden) }
}

pub async fn authorize_lesson(
    context: &UserContext,
    lesson_id: i32,
) -> Result<i32, SchoolError> {
    if lesson_id <= 0 { return Err(SchoolError::InvalidField("lesson")); }
    let tenant_key = context.tenant_id().ok_or(SchoolError::Forbidden)?;
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT l.course_id FROM lessons l INNER JOIN course_school_scopes css ON css.course_id = l.course_id INNER JOIN schools s ON s.id = css.school_id WHERE l.id = $1 AND s.tenant_key = $2 AND s.status = $3",
        _ => "SELECT l.course_id FROM lessons l INNER JOIN course_school_scopes css ON css.course_id = l.course_id INNER JOIN schools s ON s.id = css.school_id WHERE l.id = ? AND s.tenant_key = ? AND s.status = ?",
    };
    rullst::db::sqlx::query_scalar::<_, i32>(sql).bind(lesson_id).bind(tenant_key).bind("active")
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| SchoolError::Database(error.into()))?
        .filter(|course_id| *course_id > 0)
        .ok_or(SchoolError::Forbidden)
}
"##;
