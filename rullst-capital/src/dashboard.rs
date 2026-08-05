//! Revenue Dashboard & Webhook Event Inspector for Rullst Capital

use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

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
            mrr_cents: 1250000,
            arr_cents: 15000000,
            net_revenue_cents: 12100000,
            active_subscriptions: 248,
            churn_rate_percent: 1.8,
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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mock_events = vec![
            WebhookEventRecord {
                id: "evt_101".to_string(),
                provider: "stripe".to_string(),
                event_type: "checkout.session.completed".to_string(),
                status: "processed".to_string(),
                timestamp: now.saturating_sub(120),
                payload_snippet: "{\"amount_total\": 4900, \"currency\": \"usd\", \"customer\": \"cus_N123\"}".to_string(),
            },
            WebhookEventRecord {
                id: "evt_102".to_string(),
                provider: "lemonsqueezy".to_string(),
                event_type: "subscription_created".to_string(),
                status: "processed".to_string(),
                timestamp: now.saturating_sub(600),
                payload_snippet: "{\"order_id\": \"order_882\", \"status\": \"paid\"}".to_string(),
            },
        ];

        Self {
            metrics: RwLock::new(RevenueMetrics::default()),
            events: RwLock::new(mock_events),
        }
    }

    /// Retrieves current revenue metrics.
    pub fn get_metrics(&self) -> RevenueMetrics {
        self.metrics
            .read()
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    /// Records a new incoming webhook event into the audit log buffer.
    pub fn record_event(&self, event: WebhookEventRecord) {
        if let Ok(mut events) = self.events.write() {
            events.insert(0, event);
            if events.len() > 100 {
                events.pop();
            }
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
        assert!(metrics.mrr_cents > 0);

        let initial_events = manager.get_recent_events(10);
        assert!(!initial_events.is_empty());

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
        assert_eq!(updated_events[0].id, "evt_test");
    }
}
