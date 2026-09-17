use super::{CapitalError, LemonSqueezyProvider, Value};

impl LemonSqueezyProvider {
    /// Sets the merchant's actual store ID. Required for non-mock checkout;
    /// the legacy constructor remains available without inventing a store.
    pub fn with_store_id(mut self, store_id: impl Into<String>) -> Result<Self, CapitalError> {
        let store_id = store_id.into();
        positive_id(&store_id)?;
        self.store_id = Some(store_id);
        Ok(self)
    }
}

fn positive_id(value: &str) -> Result<u64, CapitalError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0 && id.to_string() == value)
        .ok_or_else(|| {
            CapitalError::ConfigurationError(
                "Lemon Squeezy store and variant IDs must be canonical positive integers".into(),
            )
        })
}

pub(super) fn request(
    store: &str,
    email: &str,
    variant: &str,
    redirect: &str,
) -> Result<Value, CapitalError> {
    positive_id(store)?;
    positive_id(variant)?;
    Ok(serde_json::json!({
        "data": {
            "type": "checkouts",
            "attributes": {
                "checkout_data": {"email": email},
                "product_options": {"redirect_url": redirect, "enabled_variants": [positive_id(variant)?]}
            },
            "relationships": {
                "store": {"data": {"type": "stores", "id": store}},
                "variant": {"data": {"type": "variants", "id": variant}}
            }
        }
    }))
}

pub(super) fn response(body: &Value, store: &str, variant: &str) -> Result<String, CapitalError> {
    let mismatch = || {
        CapitalError::from(crate::ProviderFailure::contract_mismatch(
            "lemonsqueezy",
            "create checkout",
        ))
    };
    let data = &body["data"];
    let attrs = &data["attributes"];
    if data["type"].as_str() != Some("checkouts")
        || !data["id"]
            .as_str()
            .is_some_and(|id| !id.is_empty() && id.len() <= 128)
        || attrs["store_id"].as_u64() != Some(positive_id(store)?)
        || attrs["variant_id"].as_u64() != Some(positive_id(variant)?)
    {
        return Err(mismatch());
    }
    let url = attrs["url"].as_str().ok_or_else(mismatch)?;
    crate::providers::validate_checkout_url("lemonsqueezy", url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BillingProvider;

    #[test]
    fn checkout_uses_configured_store_and_binds_both_response_identities() {
        let payload = request(
            "987",
            "person@example.invalid",
            "456",
            "https://app.example/return",
        )
        .unwrap();
        assert_eq!(
            payload["data"]["relationships"]["store"]["data"]["id"],
            "987"
        );
        assert_eq!(
            payload["data"]["attributes"]["product_options"]["enabled_variants"],
            serde_json::json!([456])
        );
        let fixture = serde_json::json!({"data":{"type":"checkouts","id":"checkout-id","attributes":{"store_id":987,"variant_id":456,"url":"https://store.lemonsqueezy.com/checkout/id"}}});
        assert!(response(&fixture, "987", "456").is_ok());
        for (store, variant) in [(1, 456), (987, 1)] {
            let mut wrong = fixture.clone();
            wrong["data"]["attributes"]["store_id"] = serde_json::json!(store);
            wrong["data"]["attributes"]["variant_id"] = serde_json::json!(variant);
            assert!(response(&wrong, "987", "456").is_err());
        }
        for id in ["", "0", "01", "+1", "variant_123", "18446744073709551616"] {
            assert!(LemonSqueezyProvider::new("", "").with_store_id(id).is_err());
            assert!(request("987", "person@example.invalid", id, "https://app.example").is_err());
        }
    }

    #[tokio::test]
    async fn missing_live_store_fails_before_network_and_mock_remains_available() {
        let live = LemonSqueezyProvider::new("fixture\ninvalid-header", "secret");
        assert!(matches!(
            live.create_checkout_session("person@example.invalid", "456", "https://app.example")
                .await,
            Err(CapitalError::ConfigurationError(_))
        ));
        for key in ["", "mock_key"] {
            assert!(
                LemonSqueezyProvider::new(key, "mock_webhook")
                    .create_checkout_session(
                        "person@example.invalid",
                        "variant_mock",
                        "https://app.example"
                    )
                    .await
                    .is_ok()
            );
        }
    }
}
