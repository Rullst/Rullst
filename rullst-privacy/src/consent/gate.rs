use super::*;

/// Trusted server clock; client timestamps must never implement this boundary.
pub trait ConsentClock: Send + Sync {
    fn now(&self) -> Result<i64, ConsentError>;
}

pub struct SystemConsentClock;
impl ConsentClock for SystemConsentClock {
    fn now(&self) -> Result<i64, ConsentError> {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ConsentError::ClockRollback)?;
        i64::try_from(elapsed.as_secs()).map_err(|_| ConsentError::ClockRollback)
    }
}

/// Authenticate the subject and enforce CSRF before entering this boundary.
/// Never cache `allows` across actions or enqueue-time/dequeue-time boundaries.
/// This gate cannot undo an external effect that already started before withdrawal.
pub struct ConsentGate<S> {
    store: S,
    development: bool,
}

impl<S: ConsentStore> ConsentGate<S> {
    pub fn new(store: S) -> Result<Self, ConsentError> {
        if store.durability() != ConsentDurability::SharedDurable {
            return Err(ConsentError::DurableStateRequired);
        }
        Ok(Self {
            store,
            development: false,
        })
    }

    pub fn for_development(store: S) -> Self {
        Self {
            store,
            development: true,
        }
    }

    pub async fn current(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
    ) -> Result<ConsentRecord, ConsentError> {
        self.current_with_clock(subject, purpose, &SystemConsentClock)
            .await
    }

    pub async fn current_with_clock(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
        clock: &impl ConsentClock,
    ) -> Result<ConsentRecord, ConsentError> {
        Ok(self.read_current(subject, purpose, clock).await?.0)
    }

    async fn read_current(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
        clock: &impl ConsentClock,
    ) -> Result<(ConsentRecord, i64), ConsentError> {
        self.production_state()?;
        let now = clock.now()?;
        if now < 0 {
            return Err(ConsentError::ClockRollback);
        }
        let record = self.store.read(subject, purpose.id(), now).await?;
        record.validate_scope(subject, purpose, now)?;
        let completed = self.completed(now, clock)?;
        Ok((record, completed))
    }

    pub async fn allows(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
    ) -> Result<bool, ConsentError> {
        self.allows_with_clock(subject, purpose, &SystemConsentClock)
            .await
    }

    pub async fn allows_with_clock(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
        clock: &impl ConsentClock,
    ) -> Result<bool, ConsentError> {
        let (record, completed) = self.read_current(subject, purpose, clock).await?;
        if record.choice() == ConsentChoice::Granted && completed >= record.valid_until() {
            // Persist an expiry observed after a delayed read, so a later clock
            // rollback cannot revive this expired grant. This is already a denial;
            // concurrent fresh consent requires a new processing check.
            self.store.read(subject, purpose.id(), completed).await?;
            return Ok(false);
        }
        Ok(record.choice() == ConsentChoice::Granted
            && record.revision() < i64::MAX as u64
            && record.version() == purpose.version()
            && completed < record.valid_until())
    }

    /// Explicit affirmative/refusal choice bound to the revision shown in the
    /// current form. `valid_until` is server-selected, positive only for grants.
    pub async fn choose(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
        submission: &ConsentSubmission,
        valid_until: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        self.choose_with_clock(
            subject,
            purpose,
            submission,
            valid_until,
            &SystemConsentClock,
        )
        .await
    }

    pub async fn choose_with_clock(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
        submission: &ConsentSubmission,
        valid_until: i64,
        clock: &impl ConsentClock,
    ) -> Result<ConsentRecord, ConsentError> {
        if &submission.displayed != purpose {
            return Err(ConsentError::BindingMismatch);
        }
        self.save(
            ConsentUpdate {
                subject: subject.clone(),
                purpose: purpose.clone(),
                expected_revision: Some(submission.revision),
                choice: submission.choice,
                now: clock.now()?,
                valid_until,
            },
            clock,
        )
        .await
    }

    /// Withdrawal is unconditional for this purpose across versions and advances
    /// the revision. Stale affirmative forms cannot reverse the completed update.
    pub async fn withdraw(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
    ) -> Result<ConsentRecord, ConsentError> {
        self.withdraw_with_clock(subject, purpose, &SystemConsentClock)
            .await
    }

    pub async fn withdraw_with_clock(
        &self,
        subject: &ConsentSubject,
        purpose: &ConsentPurpose,
        clock: &impl ConsentClock,
    ) -> Result<ConsentRecord, ConsentError> {
        self.save(
            ConsentUpdate {
                subject: subject.clone(),
                purpose: purpose.clone(),
                expected_revision: None,
                choice: ConsentChoice::Withdrawn,
                now: clock.now()?,
                valid_until: 0,
            },
            clock,
        )
        .await
    }

    async fn save(
        &self,
        update: ConsentUpdate,
        clock: &impl ConsentClock,
    ) -> Result<ConsentRecord, ConsentError> {
        self.production_state()?;
        if update.now < 0 || !state::valid_expiry(update.choice, update.now, update.valid_until) {
            return Err(ConsentError::InvalidConfiguration);
        }
        let record = self.store.update(&update).await?;
        record.validate_scope(&update.subject, &update.purpose, update.now)?;
        // A trusted adapter still must return the exact acknowledged operation.
        if record.choice() != update.choice
            || record.version() != update.purpose.version()
            || record.changed_at() != update.now
            || record.valid_until() != update.valid_until
            || record.revision() == 0
            || update
                .expected_revision
                .is_some_and(|v| v.checked_add(1) != Some(record.revision()))
        {
            return Err(ConsentError::StoreConfiguration);
        }
        let completed = self.completed(update.now, clock)?;
        if record.choice() == ConsentChoice::Granted && completed >= record.valid_until() {
            self.store
                .read(&update.subject, update.purpose.id(), completed)
                .await?;
            return Err(ConsentError::InvalidConfiguration);
        }
        Ok(record)
    }

    fn production_state(&self) -> Result<(), ConsentError> {
        if !self.development && self.store.durability() != ConsentDurability::SharedDurable {
            return Err(ConsentError::DurableStateRequired);
        }
        Ok(())
    }

    fn completed(&self, started: i64, clock: &impl ConsentClock) -> Result<i64, ConsentError> {
        self.production_state()?;
        let completed = clock.now()?;
        if completed < started {
            return Err(ConsentError::ClockRollback);
        }
        Ok(completed)
    }
}
