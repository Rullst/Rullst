//! Claim-token-guarded outbox state transitions (acknowledge and fail).

use super::sql::*;
use super::{MAX_ERROR_LEN, ensure_outside_managed_transaction, unix_now, validate_key};
use crate::{Error, Orm};

pub(super) enum Transition<'a> {
    Delivered,
    Failed {
        error: &'a str,
        max_attempts: i32,
        retry_at_epoch: i64,
        retry_delay_seconds: i64,
    },
}

pub(super) async fn transition(
    id: i64,
    claim_key: &str,
    transition: Transition<'_>,
) -> Result<bool, Error> {
    ensure_outside_managed_transaction()?;
    if id <= 0 {
        return Err(Error::Validation("outbox id must be positive".to_string()));
    }
    validate_key("claim_key", claim_key)?;
    let driver = Orm::driver()?;
    let now_epoch_seconds = unix_now()?;
    let result = match transition {
        Transition::Delivered => {
            let sql = if driver == "postgres" {
                POSTGRES_ACK
            } else {
                PORTABLE_ACK
            };
            sqlx::query(sql)
                .bind("delivered")
                .bind("")
                .bind("")
                .bind(0_i64)
                .bind(now_epoch_seconds)
                .bind(id)
                .bind("processing")
                .bind(claim_key)
                .bind(now_epoch_seconds)
                .execute(Orm::pool()?)
                .await?
        }
        Transition::Failed {
            error,
            max_attempts,
            retry_at_epoch,
            retry_delay_seconds,
        } => {
            if !(1..=100).contains(&max_attempts)
                || !(0..=86_400).contains(&retry_delay_seconds)
                || error.is_empty()
                || error.len() > MAX_ERROR_LEN
                || error.chars().any(char::is_control)
            {
                return Err(Error::Validation(
                    "outbox failure policy is outside its bound".to_string(),
                ));
            }
            let sql = if driver == "postgres" {
                POSTGRES_FAIL
            } else {
                PORTABLE_FAIL
            };
            sqlx::query(sql)
                .bind(max_attempts)
                .bind("dead_letter")
                .bind("pending")
                .bind(retry_at_epoch)
                .bind("")
                .bind("")
                .bind(0_i64)
                .bind(error)
                .bind(id)
                .bind("processing")
                .bind(claim_key)
                .bind(now_epoch_seconds)
                .execute(Orm::pool()?)
                .await?
        }
    };
    Ok(result.rows_affected() == 1)
}
