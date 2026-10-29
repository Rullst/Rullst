//! `FromRow` for [`ClaimedOutboxEvent`], written out so the crate builds
//! without SQLx's derive macros (`sqlx-macros`).

use super::ClaimedOutboxEvent;
use sqlx::{ColumnIndex, Decode, FromRow, Row, Type};

impl<'r, R> FromRow<'r, R> for ClaimedOutboxEvent
where
    R: Row,
    &'r str: ColumnIndex<R>,
    i64: Decode<'r, R::Database> + Type<R::Database>,
    i32: Decode<'r, R::Database> + Type<R::Database>,
    String: Decode<'r, R::Database> + Type<R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            stream: row.try_get("stream")?,
            event_key: row.try_get("event_key")?,
            event_kind: row.try_get("event_kind")?,
            payload_json: row.try_get("payload_json")?,
            attempts: row.try_get("attempts")?,
            claim_key: row.try_get("claim_key")?,
            claim_expires_at_epoch: row.try_get("claim_expires_at_epoch")?,
        })
    }
}
