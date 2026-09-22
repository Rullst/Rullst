use super::*;
use sqlx::Row;
use zeroize::Zeroizing;

pub(super) fn batch(limit: usize) -> Result<i64> {
    if !(1..=100).contains(&limit) {
        return Err(RecurringError::InvalidInput("batch limit"));
    }
    Ok(limit as i64)
}
impl<C: Clock> PostgresRecurringStore<C> {
    /// Atomically freezes at most 100 due occurrences, oldest due time first.
    /// Capacity errors roll back this entire tick, including schedule advancement.
    pub async fn tick(&self, limit: usize) -> Result<Vec<OccurrenceMetadata>> {
        batch(limit)?;
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_recurring_occurrences WHERE namespace=$1")
                .bind(self.config.namespace()).fetch_one(&mut *tx).await.map_err(|_| RecurringError::Storage)?;
            let mut output = Vec::new();
            for _ in 0..limit {
                let Some(row) = sqlx::query("SELECT name,generation,content,next_due FROM rullst_recurring_definitions
                    WHERE namespace=$1 AND NOT cancelled AND next_due <= $2 ORDER BY next_due,name LIMIT 1")
                    .bind(self.config.namespace()).bind(now).fetch_optional(&mut *tx).await
                    .map_err(|_| RecurringError::Storage)? else { break; };
                if count + output.len() as i64 >= self.config.occurrences as i64 { return Err(RecurringError::Capacity); }
                let name: String = row.try_get("name").map_err(|_| RecurringError::Storage)?;
                let generation: String = row.try_get("generation").map_err(|_| RecurringError::Storage)?;
                let due: i64 = row.try_get("next_due").map_err(|_| RecurringError::Storage)?;
                let bytes: Vec<u8> = row.try_get("content").map_err(|_| RecurringError::Storage)?;
                let definition: RecurringDefinition = serde_json::from_slice(&crypto::open(
                    &self.keys, self.config.namespace(), "definition", &generation, &bytes)?)
                    .map_err(|_| RecurringError::Encryption)?;
                if definition.name != name || due <= definition.first_after { return Err(RecurringError::Configuration); }
                let after = match definition.missed { MissedRunPolicy::CatchUp => due, MissedRunPolicy::Coalesce => now };
                let next = definition.next_after(after)?;
                let id = crypto::occurrence_id(self.config.namespace(), &generation, due);
                let plaintext = Zeroizing::new(serde_json::to_vec(&definition.message).map_err(|_| RecurringError::Encryption)?);
                let content = crypto::seal(&self.keys, self.config.namespace(), "occurrence", &id, &plaintext)?;
                // A caught-up occurrence receives a bounded delivery window from materialization.
                let expires = now + self.config.window_ms;
                sqlx::query("INSERT INTO rullst_recurring_occurrences
                    (namespace,id,schedule,generation,due,created,expires,content,state,available)
                    VALUES($1,$2,$3,$4,$5,$6,$7,$8,'pending',$6)")
                    .bind(self.config.namespace()).bind(&id).bind(&name).bind(&generation).bind(due)
                    .bind(now).bind(expires).bind(content).execute(&mut *tx).await.map_err(|_| RecurringError::Storage)?;
                sqlx::query("UPDATE rullst_recurring_definitions SET next_due=$1 WHERE namespace=$2 AND name=$3")
                    .bind(next).bind(self.config.namespace()).bind(&name).execute(&mut *tx).await.map_err(|_| RecurringError::Storage)?;
                output.push(OccurrenceMetadata { id, schedule: name, due, created: now, expires,
                    attempts: 0, state: OccurrenceState::Pending });
            }
            self.commit(tx, now, None).await?;
            Ok(output)
        }).await
    }
}
