//! Shared, idempotent resource quotas for users, teams, and workspaces.

use async_trait::async_trait;
use rullst_core::security::TenantContext;
use std::{collections::HashMap, future::Future, sync::Arc};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;

#[cfg(feature = "quota-sql")]
mod sql;
#[cfg(feature = "quota-sql")]
pub use sql::{SqlQuotaBackend, SqlQuotaStore};

const MAX_KIND_BYTES: usize = 32;
const MAX_ID_BYTES: usize = 128;
const MAX_FEATURE_BYTES: usize = 128;
const MAX_EVENT_KEY_BYTES: usize = 128;

/// Authoritative owner of one subscription and its shared resource counters.
#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct BillingSubject {
    kind: String,
    id: String,
}

impl BillingSubject {
    /// Creates a bounded subject such as `workspace/acme` or `team/platform`.
    pub fn try_new(kind: impl Into<String>, id: impl Into<String>) -> Result<Self, QuotaError> {
        let kind = kind.into();
        let id = id.into();
        validate_identifier("billing subject kind", &kind, MAX_KIND_BYTES)?;
        validate_identifier("billing subject ID", &id, MAX_ID_BYTES)?;
        Ok(Self { kind, id })
    }

    /// Uses an authenticated tenant as a shared subscription and quota owner.
    pub fn from_tenant(tenant: &TenantContext) -> Result<Self, QuotaError> {
        Self::try_new("tenant", tenant.tenant_id.clone())
    }

    /// Returns the bounded subject kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the authoritative subject identifier.
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Debug for BillingSubject {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BillingSubject")
            .field("kind", &self.kind)
            .field("id", &"[REDACTED]")
            .finish()
    }
}

/// One idempotent request to consume units from a shared resource limit.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QuotaRequest {
    subject: BillingSubject,
    feature: String,
    event_key: String,
    units: u64,
    limit: u64,
}

impl QuotaRequest {
    /// Creates a positive, bounded request with an application-stable event key.
    pub fn try_new(
        subject: BillingSubject,
        feature: impl Into<String>,
        event_key: impl Into<String>,
        units: u64,
        limit: u64,
    ) -> Result<Self, QuotaError> {
        let feature = feature.into();
        let event_key = event_key.into();
        validate_identifier("quota feature", &feature, MAX_FEATURE_BYTES)?;
        validate_identifier("quota event key", &event_key, MAX_EVENT_KEY_BYTES)?;
        if units == 0 || limit == 0 || units > i64::MAX as u64 || limit > i64::MAX as u64 {
            return Err(QuotaError::InvalidRequest(
                "quota units and limit must be positive signed 64-bit quantities".to_string(),
            ));
        }
        Ok(Self {
            subject,
            feature,
            event_key,
            units,
            limit,
        })
    }

    /// Returns the shared billing subject.
    pub fn subject(&self) -> &BillingSubject {
        &self.subject
    }

    /// Returns the plan feature being consumed.
    pub fn feature(&self) -> &str {
        &self.feature
    }

    /// Returns the stable application event key.
    pub fn event_key(&self) -> &str {
        &self.event_key
    }

    /// Returns requested units.
    pub fn units(&self) -> u64 {
        self.units
    }

    /// Returns the authoritative limit used for this reservation.
    pub fn limit(&self) -> u64 {
        self.limit
    }
}

impl std::fmt::Debug for QuotaRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QuotaRequest")
            .field("subject", &self.subject)
            .field("feature", &self.feature)
            .field("event_key", &"[REDACTED]")
            .field("units", &self.units)
            .field("limit", &self.limit)
            .finish()
    }
}

/// Exact evidence that a quota reservation succeeded.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QuotaGrant {
    request: QuotaRequest,
    used_after: u64,
    claim_token: String,
    replay: bool,
}

impl QuotaGrant {
    pub(crate) fn fresh(request: QuotaRequest, used_after: u64, claim_token: String) -> Self {
        Self {
            request,
            used_after,
            claim_token,
            replay: false,
        }
    }

    pub(crate) fn replay(request: QuotaRequest, used_after: u64, claim_token: String) -> Self {
        Self {
            request,
            used_after,
            claim_token,
            replay: true,
        }
    }

    /// Returns the request bound to this grant.
    pub fn request(&self) -> &QuotaRequest {
        &self.request
    }

    /// Returns shared usage after the original successful reservation.
    pub fn used_after(&self) -> u64 {
        self.used_after
    }

    /// Indicates that the event key had already reserved the same units.
    pub fn is_replay(&self) -> bool {
        self.replay
    }
}

impl std::fmt::Debug for QuotaGrant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QuotaGrant")
            .field("request", &self.request)
            .field("used_after", &self.used_after)
            .field("claim_token", &"[REDACTED]")
            .field("replay", &self.replay)
            .finish()
    }
}

/// Typed failures from quota validation, reservation, or exact release.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuotaError {
    /// A subject, feature, key, unit count, or limit is invalid.
    #[error("invalid quota request: {0}")]
    InvalidRequest(String),
    /// The shared limit cannot accommodate this request.
    #[error("quota exceeded: used {used}, requested {requested}, limit {limit}")]
    LimitExceeded {
        /// Units already reserved by the subject.
        used: u64,
        /// Units requested by this operation.
        requested: u64,
        /// Current authoritative plan limit.
        limit: u64,
    },
    /// An event key was reused with different immutable request parameters.
    #[error("quota event key was reused with different units or limit")]
    IdempotencyConflict,
    /// A release did not carry the exact secret-bound grant.
    #[error("quota grant does not match the stored reservation")]
    GrantMismatch,
    /// The durable store could not safely complete the operation.
    #[error("quota storage is unavailable")]
    StorageUnavailable,
    /// Persisted counters and claims disagree.
    #[error("quota storage contains inconsistent state")]
    CorruptState,
}

/// Static-dispatch storage contract for shared quota reservations.
#[async_trait]
pub trait QuotaStore: Send + Sync {
    /// Atomically reserves units or returns a fail-closed error.
    async fn reserve(&self, request: &QuotaRequest) -> Result<QuotaGrant, QuotaError>;

    /// Releases exactly the grant supplied by this store.
    async fn release(&self, grant: &QuotaGrant) -> Result<bool, QuotaError>;

    /// Reads current shared usage for one subject and feature.
    async fn usage(&self, subject: &BillingSubject, feature: &str) -> Result<u64, QuotaError>;
}

/// Result of executing an operation behind a quota gate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuotaExecution<T> {
    /// This call reserved quota and executed the operation.
    Executed { value: T, grant: QuotaGrant },
    /// This idempotency key was already reserved, so the operation was not repeated.
    Replay(QuotaGrant),
}

/// Failure from either reservation, the guarded operation, or compensation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum QuotaExecutionError<E> {
    /// Reservation was invalid, over limit, conflicting, or unavailable.
    #[error(transparent)]
    Quota(#[from] QuotaError),
    /// The operation failed and its reservation was released.
    #[error("quota-guarded operation failed")]
    Operation(E),
    /// The operation failed and its reservation could not be safely released.
    #[error("quota-guarded operation failed and quota compensation also failed: {release}")]
    Compensation { operation: E, release: QuotaError },
}

/// Executes application work only after a successful fresh reservation.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct QuotaGate<Store> {
    store: Store,
}

impl<Store> QuotaGate<Store>
where
    Store: QuotaStore,
{
    /// Creates a gate around an explicit statically dispatched store.
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// Returns the underlying store.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Reserves before execution, suppresses exact replays, and compensates failures.
    ///
    /// This helper is safe against quota overrun but is not an atomic database
    /// transaction with arbitrary application storage. SQL applications that
    /// need that guarantee use `SqlQuotaStore::reserve_with_transaction`.
    pub async fn execute<Operation, FutureResult, Value, Error>(
        &self,
        request: &QuotaRequest,
        operation: Operation,
    ) -> Result<QuotaExecution<Value>, QuotaExecutionError<Error>>
    where
        Operation: FnOnce() -> FutureResult,
        FutureResult: Future<Output = Result<Value, Error>>,
    {
        let grant = self.store.reserve(request).await?;
        if grant.is_replay() {
            return Ok(QuotaExecution::Replay(grant));
        }
        match operation().await {
            Ok(value) => Ok(QuotaExecution::Executed { value, grant }),
            Err(operation) => match self.store.release(&grant).await {
                Ok(true) => Err(QuotaExecutionError::Operation(operation)),
                Ok(false) => Err(QuotaExecutionError::Compensation {
                    operation,
                    release: QuotaError::CorruptState,
                }),
                Err(release) => Err(QuotaExecutionError::Compensation { operation, release }),
            },
        }
    }
}

/// Deterministic offline store with process-local shared accounting.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct InMemoryQuotaStore {
    state: Arc<Mutex<MemoryState>>,
}

#[derive(Default)]
struct MemoryState {
    usage: HashMap<UsageKey, u64>,
    claims: HashMap<ClaimKey, StoredClaim>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct UsageKey {
    subject: BillingSubject,
    feature: String,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ClaimKey {
    usage: UsageKey,
    event_key: String,
}

struct StoredClaim {
    units: u64,
    limit: u64,
    used_after: u64,
    claim_token: String,
}

impl std::fmt::Debug for InMemoryQuotaStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("InMemoryQuotaStore { state: [REDACTED] }")
    }
}

#[async_trait]
impl QuotaStore for InMemoryQuotaStore {
    async fn reserve(&self, request: &QuotaRequest) -> Result<QuotaGrant, QuotaError> {
        let usage_key = usage_key(request);
        let claim_key = ClaimKey {
            usage: usage_key.clone(),
            event_key: request.event_key.clone(),
        };
        let mut state = self.state.lock().await;
        if let Some(claim) = state.claims.get(&claim_key) {
            validate_replay(claim.units, claim.limit, request)?;
            return Ok(QuotaGrant::replay(
                request.clone(),
                claim.used_after,
                claim.claim_token.clone(),
            ));
        }
        let used = state.usage.get(&usage_key).copied().unwrap_or(0);
        let used_after = used
            .checked_add(request.units)
            .ok_or(QuotaError::LimitExceeded {
                used,
                requested: request.units,
                limit: request.limit,
            })?;
        if used_after > request.limit {
            return Err(QuotaError::LimitExceeded {
                used,
                requested: request.units,
                limit: request.limit,
            });
        }
        let claim_token = random_claim_token()?;
        state.usage.insert(usage_key, used_after);
        state.claims.insert(
            claim_key,
            StoredClaim {
                units: request.units,
                limit: request.limit,
                used_after,
                claim_token: claim_token.clone(),
            },
        );
        Ok(QuotaGrant::fresh(request.clone(), used_after, claim_token))
    }

    async fn release(&self, grant: &QuotaGrant) -> Result<bool, QuotaError> {
        let usage_key = usage_key(&grant.request);
        let claim_key = ClaimKey {
            usage: usage_key.clone(),
            event_key: grant.request.event_key.clone(),
        };
        let mut state = self.state.lock().await;
        let Some(claim) = state.claims.get(&claim_key) else {
            return Ok(false);
        };
        if !tokens_match(&claim.claim_token, &grant.claim_token) {
            return Err(QuotaError::GrantMismatch);
        }
        let units = claim.units;
        let used = state
            .usage
            .get(&usage_key)
            .copied()
            .ok_or(QuotaError::CorruptState)?;
        let remaining = used.checked_sub(units).ok_or(QuotaError::CorruptState)?;
        state.claims.remove(&claim_key);
        if remaining == 0 {
            state.usage.remove(&usage_key);
        } else {
            state.usage.insert(usage_key, remaining);
        }
        Ok(true)
    }

    async fn usage(&self, subject: &BillingSubject, feature: &str) -> Result<u64, QuotaError> {
        validate_identifier("quota feature", feature, MAX_FEATURE_BYTES)?;
        let key = UsageKey {
            subject: subject.clone(),
            feature: feature.to_string(),
        };
        Ok(self
            .state
            .lock()
            .await
            .usage
            .get(&key)
            .copied()
            .unwrap_or(0))
    }
}

fn usage_key(request: &QuotaRequest) -> UsageKey {
    UsageKey {
        subject: request.subject.clone(),
        feature: request.feature.clone(),
    }
}

pub(crate) fn validate_replay(
    units: u64,
    limit: u64,
    request: &QuotaRequest,
) -> Result<(), QuotaError> {
    if units == request.units && limit == request.limit {
        Ok(())
    } else {
        Err(QuotaError::IdempotencyConflict)
    }
}

pub(crate) fn random_claim_token() -> Result<String, QuotaError> {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut bytes = [0_u8; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| QuotaError::StorageUnavailable)?;
    Ok(hex::encode(bytes))
}

pub(crate) fn tokens_match(left: &str, right: &str) -> bool {
    left.len() == right.len() && left.as_bytes().ct_eq(right.as_bytes()).into()
}

fn validate_identifier(label: &str, value: &str, max_bytes: usize) -> Result<(), QuotaError> {
    if value.is_empty()
        || value.len() > max_bytes
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(QuotaError::InvalidRequest(format!(
            "{label} must contain 1 to {max_bytes} ASCII identifier bytes"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
