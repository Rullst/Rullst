pub mod blueprint;
pub mod column;
pub mod join;
pub mod migration;
pub mod schema_builder;
pub mod validation;

#[cfg(test)]
mod tests;

pub use blueprint::Blueprint;
pub use column::{Column, ColumnDefault};
pub use join::JoinClause;
pub use migration::{Migration, run_artisan, run_artisan_with_args};
pub use schema_builder::Schema;
pub use validation::{ALLOWED_OPERATORS, validate_identifier, validate_table_name};

pub trait SubqueryBuilder {
    fn to_sql(&self) -> String;
    fn bindings(&self) -> &Vec<crate::RullstValue>;
}

pub static QUERY_LOGGING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static MAX_QUERY_LIMIT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(1000);
pub static QUERY_TIMEOUT_SECS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(30);

pub fn enable_query_log() {
    QUERY_LOGGING.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn disable_query_log() {
    QUERY_LOGGING.store(false, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_query_log_enabled() -> bool {
    QUERY_LOGGING.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn set_max_query_limit(limit: usize) {
    MAX_QUERY_LIMIT.store(limit, std::sync::atomic::Ordering::SeqCst);
}

pub fn get_max_query_limit() -> Option<usize> {
    let limit = MAX_QUERY_LIMIT.load(std::sync::atomic::Ordering::SeqCst);
    if limit == 0 { None } else { Some(limit) }
}

pub fn set_query_timeout(secs: u64) {
    QUERY_TIMEOUT_SECS.store(secs, std::sync::atomic::Ordering::SeqCst);
}

pub fn get_query_timeout() -> Option<std::time::Duration> {
    let secs = QUERY_TIMEOUT_SECS.load(std::sync::atomic::Ordering::SeqCst);
    if secs == 0 {
        None
    } else {
        Some(std::time::Duration::from_secs(secs))
    }
}
