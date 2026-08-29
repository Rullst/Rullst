//! Revenue Dashboard & Webhook Event Inspector for Rullst Capital

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Application-supplied revenue analytics metrics payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RevenueMetrics {
    /// Monthly Recurring Revenue in cents.
    pub mrr_cents: u64,
    /// Annual Recurring Revenue in cents.
    pub arr_cents: u64,
    /// Net Revenue in cents.
    pub net_revenue_cents: u64,
    /// Total active paying subscriber count.
    pub active_subscriptions: u32,
    /// Estimated monthly churn rate percentage.
    pub churn_rate_percent: f64,
}

impl Default for RevenueMetrics {
    fn default() -> Self {
        Self {
            mrr_cents: 0,
            arr_cents: 0,
            net_revenue_cents: 0,
            active_subscriptions: 0,
            churn_rate_percent: 0.0,
        }
    }
}

/// Application-supplied record describing the result of a webhook processing
/// path. Constructing this value does not perform signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventRecord {
    /// Unique event identifier.
    pub id: String,
    /// Provider name ("stripe" or "lemonsqueezy").
    pub provider: String,
    /// Webhook event type (e.g. "invoice.payment_succeeded", "subscription_created").
    pub event_type: String,
    /// Execution status ("processed", "failed", "signature_invalid").
    pub status: String,
    /// Timestamp of receipt in seconds.
    pub timestamp: u64,
    /// Raw or truncated JSON payload snippet.
    pub payload_snippet: String,
}

/// Central manager for Rullst Capital analytics and webhook audit logs.
pub struct RevenueDashboardManager {
    metrics: RwLock<RevenueMetrics>,
    events: RwLock<Vec<WebhookEventRecord>>,
}

impl Default for RevenueDashboardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RevenueDashboardManager {
    /// Creates a new `RevenueDashboardManager` with default initial state.
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(RevenueMetrics::default()),
            events: RwLock::new(Vec::new()),
        }
    }

    /// Retrieves current revenue metrics.
    pub fn get_metrics(&self) -> RevenueMetrics {
        match self.metrics.read() {
            Ok(metrics) => metrics.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Updates current revenue metrics.
    pub fn update_metrics(&self, new_metrics: RevenueMetrics) {
        let mut metrics = match self.metrics.write() {
            Ok(metrics) => metrics,
            Err(poisoned) => poisoned.into_inner(),
        };
        *metrics = new_metrics;
    }

    /// Records a webhook event in the bounded process-local inspection buffer.
    ///
    /// An event type does not contain enough authoritative price, currency,
    /// refund, fee or subscription state to derive financial metrics. Call
    /// [`Self::update_metrics`] with values computed from the application's
    /// durable billing source instead.
    pub fn record_event(&self, mut event: WebhookEventRecord) {
        event.id = bounded_text(event.id, 128);
        event.provider = bounded_text(event.provider, 64);
        event.event_type = bounded_text(event.event_type, 128);
        event.status = bounded_text(event.status, 64);
        event.payload_snippet = bounded_text(event.payload_snippet, 2_048);

        let mut events = match self.events.write() {
            Ok(events) => events,
            Err(poisoned) => poisoned.into_inner(),
        };
        events.insert(0, event);
        if events.len() > 100 {
            events.pop();
        }
    }

    /// Lists recent webhook events (up to limit).
    pub fn get_recent_events(&self, limit: usize) -> Vec<WebhookEventRecord> {
        let events = match self.events.read() {
            Ok(events) => events,
            Err(poisoned) => poisoned.into_inner(),
        };
        events.iter().take(limit.min(100)).cloned().collect()
    }
}

fn bounded_text(value: String, maximum_chars: usize) -> String {
    if value.chars().count() <= maximum_chars {
        value
    } else {
        value.chars().take(maximum_chars).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revenue_dashboard_manager() {
        let manager = RevenueDashboardManager::new();
        let metrics = manager.get_metrics();
        assert_eq!(metrics.mrr_cents, 0);

        let initial_events = manager.get_recent_events(10);
        assert!(initial_events.is_empty());

        let new_evt = WebhookEventRecord {
            id: "evt_test".to_string(),
            provider: "stripe".to_string(),
            event_type: "invoice.paid".to_string(),
            status: "processed".to_string(),
            timestamp: 1000,
            payload_snippet: "{}".to_string(),
        };

        manager.record_event(new_evt);
        let updated_events = manager.get_recent_events(10);
        assert_eq!(updated_events.len(), 1);
        assert_eq!(updated_events[0].id, "evt_test");
        assert_eq!(manager.get_metrics(), RevenueMetrics::default());

        manager.record_event(WebhookEventRecord {
            id: "x".repeat(200),
            provider: "provider".to_string(),
            event_type: "type".to_string(),
            status: "observed".to_string(),
            timestamp: 1001,
            payload_snippet: "é".repeat(3_000),
        });
        let bounded = manager.get_recent_events(1);
        assert_eq!(bounded[0].id.chars().count(), 128);
        assert_eq!(bounded[0].payload_snippet.chars().count(), 2_048);
    }
}
