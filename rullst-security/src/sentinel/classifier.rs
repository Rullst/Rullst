use super::SentinelError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_WINDOW_SECONDS: u64 = 60 * 60;
const MAX_OBSERVATION_COUNT: u64 = 1_000_000_000;

/// Evidence category inferred from one caller-supplied aggregate window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ThreatPattern {
    /// Many failed authentications spanning multiple account identifiers.
    CredentialStuffing,
    /// High request volume traversing many distinct paths.
    ApiScraping,
    /// Correlated automation observed across many distinct network sources.
    DistributedAutomation,
}

/// Bounded action recommended by the deterministic classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SentinelAction {
    /// No named threshold was reached.
    Observe,
    /// Ask the caller to complete the separately verified proof-of-work flow.
    ProofOfWork,
}

/// Validated aggregate signals from one application-defined observation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SentinelObservation {
    window_seconds: u64,
    total_requests: u64,
    failed_auth_attempts: u64,
    distinct_accounts: u64,
    distinct_paths: u64,
    distinct_sources: u64,
    correlated_sources: u64,
}

impl SentinelObservation {
    /// Builds a bounded internally consistent aggregate window.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        window: Duration,
        total_requests: u64,
        failed_auth_attempts: u64,
        distinct_accounts: u64,
        distinct_paths: u64,
        distinct_sources: u64,
        correlated_sources: u64,
    ) -> Result<Self, SentinelError> {
        let window_seconds = window.as_secs();
        let counts = [
            total_requests,
            failed_auth_attempts,
            distinct_accounts,
            distinct_paths,
            distinct_sources,
            correlated_sources,
        ];
        if !(1..=MAX_WINDOW_SECONDS).contains(&window_seconds) {
            return Err(SentinelError::InvalidObservation("window"));
        }
        if total_requests == 0
            || counts
                .into_iter()
                .any(|count| count > MAX_OBSERVATION_COUNT)
        {
            return Err(SentinelError::InvalidObservation("request count"));
        }
        if failed_auth_attempts > total_requests
            || distinct_accounts > failed_auth_attempts
            || distinct_paths > total_requests
            || distinct_sources == 0
            || distinct_sources > total_requests
            || correlated_sources > distinct_sources
        {
            return Err(SentinelError::InvalidObservation("inconsistent counts"));
        }
        Ok(Self {
            window_seconds,
            total_requests,
            failed_auth_attempts,
            distinct_accounts,
            distinct_paths,
            distinct_sources,
            correlated_sources,
        })
    }

    pub fn window_seconds(self) -> u64 {
        self.window_seconds
    }

    pub fn total_requests(self) -> u64 {
        self.total_requests
    }

    pub fn failed_auth_attempts(self) -> u64 {
        self.failed_auth_attempts
    }

    pub fn distinct_accounts(self) -> u64 {
        self.distinct_accounts
    }

    pub fn distinct_paths(self) -> u64 {
        self.distinct_paths
    }

    pub fn distinct_sources(self) -> u64 {
        self.distinct_sources
    }

    pub fn correlated_sources(self) -> u64 {
        self.correlated_sources
    }
}

/// Transparent thresholds for the three supported deterministic patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SentinelPolicy {
    credential_failures_per_minute: u64,
    credential_distinct_accounts: u64,
    credential_failure_ratio_bps: u16,
    scraping_requests_per_minute: u64,
    scraping_distinct_paths: u64,
    distributed_requests_per_minute: u64,
    distributed_distinct_sources: u64,
    distributed_correlated_sources: u64,
}

impl Default for SentinelPolicy {
    fn default() -> Self {
        Self {
            credential_failures_per_minute: 20,
            credential_distinct_accounts: 5,
            credential_failure_ratio_bps: 7_000,
            scraping_requests_per_minute: 240,
            scraping_distinct_paths: 40,
            distributed_requests_per_minute: 300,
            distributed_distinct_sources: 20,
            distributed_correlated_sources: 10,
        }
    }
}

impl SentinelPolicy {
    /// Replaces the credential-stuffing thresholds.
    pub fn try_with_credential_stuffing(
        mut self,
        failures_per_minute: u64,
        distinct_accounts: u64,
        failure_ratio_bps: u16,
    ) -> Result<Self, SentinelError> {
        validate_threshold(failures_per_minute, "credential failures")?;
        validate_threshold(distinct_accounts, "credential accounts")?;
        if !(1..=10_000).contains(&failure_ratio_bps) {
            return Err(SentinelError::InvalidConfiguration(
                "credential failure ratio",
            ));
        }
        self.credential_failures_per_minute = failures_per_minute;
        self.credential_distinct_accounts = distinct_accounts;
        self.credential_failure_ratio_bps = failure_ratio_bps;
        Ok(self)
    }

    /// Replaces the API-scraping thresholds.
    pub fn try_with_api_scraping(
        mut self,
        requests_per_minute: u64,
        distinct_paths: u64,
    ) -> Result<Self, SentinelError> {
        validate_threshold(requests_per_minute, "scraping requests")?;
        validate_threshold(distinct_paths, "scraping paths")?;
        self.scraping_requests_per_minute = requests_per_minute;
        self.scraping_distinct_paths = distinct_paths;
        Ok(self)
    }

    /// Replaces the distributed-automation thresholds.
    pub fn try_with_distributed_automation(
        mut self,
        requests_per_minute: u64,
        distinct_sources: u64,
        correlated_sources: u64,
    ) -> Result<Self, SentinelError> {
        validate_threshold(requests_per_minute, "distributed requests")?;
        validate_threshold(distinct_sources, "distributed sources")?;
        validate_threshold(correlated_sources, "correlated sources")?;
        if correlated_sources > distinct_sources {
            return Err(SentinelError::InvalidConfiguration(
                "correlated sources exceed distinct sources",
            ));
        }
        self.distributed_requests_per_minute = requests_per_minute;
        self.distributed_distinct_sources = distinct_sources;
        self.distributed_correlated_sources = correlated_sources;
        Ok(self)
    }
}

/// Deterministic classification result; it is evidence, not attacker attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SentinelAssessment {
    patterns: Vec<ThreatPattern>,
    risk_score: u8,
    action: SentinelAction,
}

impl SentinelAssessment {
    pub fn patterns(&self) -> &[ThreatPattern] {
        &self.patterns
    }

    pub fn risk_score(&self) -> u8 {
        self.risk_score
    }

    pub fn action(&self) -> SentinelAction {
        self.action
    }
}

/// Pure bounded classifier over trusted aggregate observations.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ThreatClassifier {
    policy: SentinelPolicy,
}

impl ThreatClassifier {
    pub fn new(policy: SentinelPolicy) -> Self {
        Self { policy }
    }

    pub fn assess(&self, observation: SentinelObservation) -> SentinelAssessment {
        let request_rate = per_minute(observation.total_requests, observation.window_seconds);
        let failure_rate = per_minute(observation.failed_auth_attempts, observation.window_seconds);
        let failure_ratio =
            observation.failed_auth_attempts.saturating_mul(10_000) / observation.total_requests;
        let mut patterns = Vec::with_capacity(3);
        let mut risk_score = 0;

        if failure_rate >= self.policy.credential_failures_per_minute
            && observation.distinct_accounts >= self.policy.credential_distinct_accounts
            && failure_ratio >= u64::from(self.policy.credential_failure_ratio_bps)
        {
            patterns.push(ThreatPattern::CredentialStuffing);
            risk_score = risk_score.max(75);
        }
        if request_rate >= self.policy.scraping_requests_per_minute
            && observation.distinct_paths >= self.policy.scraping_distinct_paths
        {
            patterns.push(ThreatPattern::ApiScraping);
            risk_score = risk_score.max(60);
        }
        if request_rate >= self.policy.distributed_requests_per_minute
            && observation.distinct_sources >= self.policy.distributed_distinct_sources
            && observation.correlated_sources >= self.policy.distributed_correlated_sources
        {
            patterns.push(ThreatPattern::DistributedAutomation);
            risk_score = risk_score.max(80);
        }
        let action = if patterns.is_empty() {
            SentinelAction::Observe
        } else {
            SentinelAction::ProofOfWork
        };
        SentinelAssessment {
            patterns,
            risk_score,
            action,
        }
    }
}

impl Default for ThreatClassifier {
    fn default() -> Self {
        Self::new(SentinelPolicy::default())
    }
}

fn validate_threshold(value: u64, name: &'static str) -> Result<(), SentinelError> {
    if value == 0 || value > MAX_OBSERVATION_COUNT {
        Err(SentinelError::InvalidConfiguration(name))
    } else {
        Ok(())
    }
}

fn per_minute(count: u64, window_seconds: u64) -> u64 {
    count.saturating_mul(60).div_ceil(window_seconds)
}
