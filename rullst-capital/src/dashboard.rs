//! Revenue Dashboard & Webhook Event Inspector for Rullst Capital

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Real-time revenue analytics metrics payload.
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

/// Record of a processed payment provider webhook event (Stripe or LemonSqueezy).
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
        self.metrics.read().map(|m| m.clone()).unwrap_or_default()
    }

    /// Updates current revenue metrics.
    pub fn update_metrics(&self, new_metrics: RevenueMetrics) {
        if let Ok(mut m) = self.metrics.write() {
            *m = new_metrics;
        }
    }

    /// Records a new incoming webhook event into the audit log buffer and updates live metrics.
    pub fn record_event(&self, event: WebhookEventRecord) {
        let is_processed = event.status == "processed";
        let is_sub_or_payment = event.event_type.contains("subscription")
            || event.event_type.contains("payment_succeeded");

        if let Ok(mut events) = self.events.write() {
            events.insert(0, event);
            if events.len() > 100 {
                events.pop();
            }
        }

        // Dynamically update metrics based on payment events
        if is_processed
            && is_sub_or_payment
            && let Ok(mut m) = self.metrics.write()
        {
            m.active_subscriptions += 1;
            m.mrr_cents += 2900; // default tier $29.00
            m.arr_cents = m.mrr_cents * 12;
            m.net_revenue_cents += 2816; // net after ~2.9% fees
        }
    }

    /// Lists recent webhook events (up to limit).
    pub fn get_recent_events(&self, limit: usize) -> Vec<WebhookEventRecord> {
        self.events
            .read()
            .map(|e| e.iter().take(limit).cloned().collect())
            .unwrap_or_default()
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
    }
}
