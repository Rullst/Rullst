use super::*;
use sqlx::{Row, postgres::PgRow};
use zeroize::Zeroizing;

pub(super) fn name(value: &str) -> Result<()> {
    crate::validation::validate_route_identifier("schedule name", value, 128)
        .map_err(|_| RecurringError::InvalidInput("schedule name"))
}
pub(super) fn metadata(row: &PgRow) -> Result<RecurringMetadata> {
    Ok(RecurringMetadata {
        name: row.try_get("name").map_err(|_| RecurringError::Storage)?,
        generation: row
            .try_get("generation")
            .map_err(|_| RecurringError::Storage)?,
        created: row
            .try_get("created")
            .map_err(|_| RecurringError::Storage)?,
        next_due: row
            .try_get("next_due")
            .map_err(|_| RecurringError::Storage)?,
        cancelled: row
            .try_get("cancelled")
            .map_err(|_| RecurringError::Storage)?,
    })
}
impl<C: Clock> PostgresRecurringStore<C> {
    /// Idempotently stores an immutable definition. A cancelled name cannot be reactivated.
    pub async fn create(&self, definition: RecurringDefinition) -> Result<RecurringMetadata> {
        definition.validate()?;
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            if let Some(row) = sqlx::query(
                "SELECT name,generation,created,next_due,cancelled,content
                FROM rullst_recurring_definitions WHERE namespace=$1 AND name=$2",
            )
            .bind(self.config.namespace())
            .bind(&definition.name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?
            {
                let metadata = metadata(&row)?;
                let bytes: Vec<u8> = row
                    .try_get("content")
                    .map_err(|_| RecurringError::Storage)?;
                let stored: RecurringDefinition = serde_json::from_slice(&crypto::open(
                    &self.keys,
                    self.config.namespace(),
                    "definition",
                    &metadata.generation,
                    &bytes,
                )?)
                .map_err(|_| RecurringError::Encryption)?;
                if stored != definition {
                    return Err(RecurringError::Conflict);
                }
                self.commit(tx, now, None).await?;
                return Ok(metadata);
            }
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM rullst_recurring_definitions WHERE namespace=$1",
            )
            .bind(self.config.namespace())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?;
            if count >= self.config.schedules as i64 {
                return Err(RecurringError::Capacity);
            }
            let generation = crypto::random_token()?.to_string();
            let next_due = definition.next_after(definition.first_after)?;
            let plaintext = Zeroizing::new(
                serde_json::to_vec(&definition).map_err(|_| RecurringError::Encryption)?,
            );
            let content = crypto::seal(
                &self.keys,
                self.config.namespace(),
                "definition",
                &generation,
                &plaintext,
            )?;
            sqlx::query(
                "INSERT INTO rullst_recurring_definitions
                (namespace,name,generation,content,created,next_due) VALUES($1,$2,$3,$4,$5,$6)",
            )
            .bind(self.config.namespace())
            .bind(&definition.name)
            .bind(&generation)
            .bind(content)
            .bind(now)
            .bind(next_due)
            .execute(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?;
            self.commit(tx, now, None).await?;
            Ok(RecurringMetadata {
                name: definition.name,
                generation,
                created: now,
                next_due,
                cancelled: false,
            })
        })
        .await
    }
    /// Permanently cancels this generation and fences every unpublished occurrence.
    /// Already in-flight broker publication cannot be recalled.
    pub async fn cancel(&self, schedule_name: impl Into<String>) -> Result<()> {
        let schedule_name = schedule_name.into();
        name(&schedule_name)?;
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            let affected = sqlx::query(
                "UPDATE rullst_recurring_definitions SET cancelled=TRUE,next_due=NULL
                WHERE namespace=$1 AND name=$2",
            )
            .bind(self.config.namespace())
            .bind(&schedule_name)
            .execute(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?
            .rows_affected();
            if affected != 1 {
                return Err(RecurringError::NotFound);
            }
            sqlx::query(
                "UPDATE rullst_recurring_occurrences SET state='cancelled',content=NULL,
                lease_hash=NULL,lease_until=NULL,terminal_at=$1
                WHERE namespace=$2 AND schedule=$3 AND state IN ('pending','leased','dead')",
            )
            .bind(now)
            .bind(self.config.namespace())
            .bind(&schedule_name)
            .execute(&mut *tx)
            .await
            .map_err(|_| RecurringError::Storage)?;
            self.commit(tx, now, None).await
        })
        .await
    }
}
