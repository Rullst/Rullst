use crate::models::category::Category;
use crate::models::course::Course;
use crate::models::lesson::Lesson;
use crate::pages::lms;
use rullst::response::Html;
use rullst::server::{Extension, IntoResponse, Path, Query, Response, StatusCode};
use serde::Deserialize;

const MAX_CATALOG_QUERY_CHARS: usize = 100;
const MAX_CATALOG_RESULTS: usize = 100;
const MAX_COURSE_LESSONS: usize = 500;

#[derive(Debug, Default, Deserialize)]
pub struct CatalogQuery {
    pub q: Option<String>,
    pub category: Option<i32>,
}

#[derive(Debug)]
pub enum CatalogError {
    InvalidQuery,
    Database(rullst_orm::Error),
}

impl From<rullst_orm::Error> for CatalogError {
    fn from(error: rullst_orm::Error) -> Self {
        Self::Database(error)
    }
}

pub fn normalize_catalog_query(
    query: &CatalogQuery,
) -> Result<(String, Option<i32>), CatalogError> {
    let normalized = query.q.as_deref().unwrap_or_default().trim();
    if normalized.chars().count() > MAX_CATALOG_QUERY_CHARS
        || normalized.chars().any(char::is_control)
        || query.category.is_some_and(|category| category <= 0)
    {
        return Err(CatalogError::InvalidQuery);
    }
    Ok((normalized.to_owned(), query.category))
}

pub async fn search_courses(
    query: &CatalogQuery,
) -> Result<(String, Option<i32>, Vec<Course>), CatalogError> {
    let (normalized, category) = normalize_catalog_query(query)?;
    let mut builder = Course::query();
    if let Some(category_id) = category {
        builder = builder.where_eq("category_id", category_id);
    }
    if !normalized.is_empty() {
        builder = builder.where_like("title", format!("%{normalized}%"));
    }
    let courses = builder
        .order_by("title")
        .limit(MAX_CATALOG_RESULTS)
        .get()
        .await?;
    Ok((normalized, category, courses))
}

fn error_response(error: CatalogError) -> Response {
    match error {
        CatalogError::InvalidQuery => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Catalog query must contain at most 100 visible characters and a positive category",
        )
            .into_response(),
        CatalogError::Database(error) => {
            eprintln!("LMS catalog query failed: {error}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Catalog temporarily unavailable",
            )
                .into_response()
        }
    }
}

pub async fn index(
    Query(query): Query<CatalogQuery>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> Response {
    let categories = match Category::query()
        .order_by("name")
        .limit(MAX_CATALOG_RESULTS)
        .get()
        .await
    {
        Ok(categories) => categories,
        Err(error) => return error_response(error.into()),
    };
    let (normalized, category, courses) = match search_courses(&query).await {
        Ok(result) => result,
        Err(error) => return error_response(error),
    };
    let nonce = csp_nonce
        .as_ref()
        .map(|Extension(value)| value.as_str())
        .unwrap_or_default();
    Html(lms::index_page(
        categories,
        courses,
        &normalized,
        category,
        nonce,
    ))
    .into_response()
}

pub async fn show_course(
    Path(id): Path<i32>,
    csrf: Option<Extension<rullst::security::CsrfToken>>,
    csp_nonce: Option<Extension<rullst::security::CspNonce>>,
) -> Response {
    if id <= 0 {
        return (StatusCode::BAD_REQUEST, "Course id must be positive").into_response();
    }
    let course = match Course::find(id).await {
        Ok(Some(course)) => course,
        Ok(None) => return (StatusCode::NOT_FOUND, "Course not found").into_response(),
        Err(error) => return error_response(error.into()),
    };
    let lessons = match Lesson::query()
        .where_eq("course_id", id)
        .order_by("id")
        .limit(MAX_COURSE_LESSONS)
        .get()
        .await
    {
        Ok(lessons) => lessons,
        Err(error) => return error_response(error.into()),
    };
    let csrf_token = csrf
        .as_ref()
        .map(|Extension(token)| token.as_str())
        .unwrap_or_default();
    let nonce = csp_nonce
        .as_ref()
        .map(|Extension(value)| value.as_str())
        .unwrap_or_default();
    Html(lms::course_detail_page(
        course,
        lessons,
        csrf_token,
        nonce,
    ))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::{CatalogError, CatalogQuery, normalize_catalog_query};

    #[test]
    fn catalog_query_is_bounded_and_category_is_positive() {
        let valid = normalize_catalog_query(&CatalogQuery {
            q: Some("  Rust systems  ".to_string()),
            category: Some(2),
        });
        assert!(matches!(
            valid,
            Ok((query, Some(2))) if query == "Rust systems"
        ));
        assert!(matches!(
            normalize_catalog_query(&CatalogQuery {
                q: Some("x".repeat(101)),
                category: None,
            }),
            Err(CatalogError::InvalidQuery)
        ));
        assert!(matches!(
            normalize_catalog_query(&CatalogQuery {
                q: None,
                category: Some(0),
            }),
            Err(CatalogError::InvalidQuery)
        ));
    }
}
