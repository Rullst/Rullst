use super::{
    EntitlementError, EntitlementMode, EntitlementScope, EntitlementStatus, EntitlementStore,
    identifier,
};

/// Server-owned feature policy; duplicate/empty/oversized plan lists are invalid.
#[derive(Clone)]
pub struct EntitlementPolicy {
    feature: String,
    plans: Vec<String>,
    mode: EntitlementMode,
    max_age_seconds: u16,
}

impl EntitlementPolicy {
    pub fn new(
        feature: impl Into<String>,
        plans: impl IntoIterator<Item = impl Into<String>>,
        mode: EntitlementMode,
        max_age_seconds: u16,
    ) -> Result<Self, EntitlementError> {
        let feature = feature.into();
        identifier(&feature, 128)?;
        if mode == EntitlementMode::Mock || !(1..=300).contains(&max_age_seconds) {
            return Err(EntitlementError::Invalid);
        }
        let mut allowed = Vec::new();
        for plan in plans {
            if allowed.len() >= 64 {
                return Err(EntitlementError::Invalid);
            }
            let plan = plan.into();
            identifier(&plan, 200)?;
            if allowed.contains(&plan) {
                return Err(EntitlementError::Invalid);
            }
            allowed.push(plan);
        }
        if allowed.is_empty() {
            return Err(EntitlementError::Invalid);
        }
        Ok(Self {
            feature,
            plans: allowed,
            mode,
            max_age_seconds,
        })
    }

    pub fn feature(&self) -> &str {
        &self.feature
    }
}

impl std::fmt::Debug for EntitlementPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntitlementPolicy")
            .field("mode", &self.mode)
            .field("max_age_seconds", &self.max_age_seconds)
            .finish_non_exhaustive()
    }
}

/// Server time, never a client timestamp. Adapters may enforce a durable clock
/// high-water mark; this gate also rejects rollback within its own read.
pub trait EntitlementClock: Sync {
    fn unix_seconds(&self) -> Result<i64, EntitlementError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemEntitlementClock;

impl EntitlementClock for SystemEntitlementClock {
    fn unix_seconds(&self) -> Result<i64, EntitlementError> {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| EntitlementError::Clock)?;
        i64::try_from(elapsed.as_secs()).map_err(|_| EntitlementError::Clock)
    }
}

/// Per-action read authorization. Success is deliberately not a reusable token.
#[derive(Debug, Clone)]
pub struct EntitlementGate {
    policy: EntitlementPolicy,
}

impl EntitlementGate {
    pub fn new(policy: EntitlementPolicy) -> Self {
        Self { policy }
    }

    pub fn feature(&self) -> &str {
        self.policy.feature()
    }

    pub async fn authorize(
        &self,
        scope: &EntitlementScope,
        store: &impl EntitlementStore,
    ) -> Result<(), EntitlementError> {
        self.authorize_with_clock(scope, store, &SystemEntitlementClock)
            .await
    }

    /// Explicit clock injection for trusted database clocks and deterministic tests.
    pub async fn authorize_with_clock(
        &self,
        scope: &EntitlementScope,
        store: &impl EntitlementStore,
        clock: &impl EntitlementClock,
    ) -> Result<(), EntitlementError> {
        let before = clock.unix_seconds()?;
        if before < 0 {
            return Err(EntitlementError::Clock);
        }
        let snapshot = store.current(scope).await?;
        let after = clock.unix_seconds()?;
        if after < before {
            return Err(EntitlementError::Clock);
        }
        let snapshot = snapshot.ok_or(EntitlementError::Denied)?;
        if &snapshot.scope != scope
            || snapshot.mode == EntitlementMode::Mock
            || snapshot.mode != self.policy.mode
            || snapshot.status != EntitlementStatus::Active
            || !self.policy.plans.contains(&snapshot.plan)
            || snapshot.observed_at > after
            || after.saturating_sub(snapshot.observed_at) >= i64::from(self.policy.max_age_seconds)
            || after >= snapshot.valid_until
        {
            return Err(EntitlementError::Denied);
        }
        Ok(())
    }
}
