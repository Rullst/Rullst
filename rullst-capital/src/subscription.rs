//! Validated subscription-management values shared by billing providers.

use crate::CapitalError;

const MAX_COUPON_CODE_BYTES: usize = 256;
const MAX_PROVIDER_ID_BYTES: usize = 255;

/// Maximum relative trial extension accepted by the portable API.
pub const MAX_TRIAL_EXTENSION_DAYS: u16 = 730;

/// A bounded provider coupon identifier.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CouponCode(String);

impl CouponCode {
    /// Validates a coupon identifier before it reaches a provider request.
    pub fn try_new(value: impl Into<String>) -> Result<Self, CapitalError> {
        let value = value.into();
        validate_ascii_identifier("coupon code", &value, MAX_COUPON_CODE_BYTES)?;
        Ok(Self(value))
    }

    /// Returns the provider coupon identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for CouponCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CouponCode([REDACTED])")
    }
}

/// A positive, bounded relative extension resolved against a trusted clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TrialExtension {
    days: u16,
    ends_at: i64,
}

impl TrialExtension {
    /// Resolves the extension against the current UTC clock.
    pub fn from_days(days: u16) -> Result<Self, CapitalError> {
        Self::from_days_at(days, chrono::Utc::now().timestamp())
    }

    /// Resolves the extension against an explicit trusted clock for tests and workers.
    pub fn from_days_at(days: u16, now: i64) -> Result<Self, CapitalError> {
        if days == 0 || days > MAX_TRIAL_EXTENSION_DAYS || now < 0 {
            return Err(CapitalError::SubscriptionError(format!(
                "trial extension must contain 1 to {MAX_TRIAL_EXTENSION_DAYS} days and a non-negative clock"
            )));
        }
        let seconds = i64::from(days)
            .checked_mul(24 * 60 * 60)
            .ok_or_else(trial_overflow_error)?;
        let ends_at = now.checked_add(seconds).ok_or_else(trial_overflow_error)?;
        Ok(Self { days, ends_at })
    }

    /// Returns the requested number of whole days.
    pub fn days(self) -> u16 {
        self.days
    }

    /// Returns the resolved Unix expiration timestamp.
    pub fn ends_at(self) -> i64 {
        self.ends_at
    }
}

pub(crate) fn validate_provider_subscription_id(value: &str) -> Result<(), CapitalError> {
    validate_ascii_identifier("subscription ID", value, MAX_PROVIDER_ID_BYTES)
}

pub(crate) fn validate_coupon_code(value: &str) -> Result<CouponCode, CapitalError> {
    CouponCode::try_new(value)
}

pub(crate) fn validate_trial_end(value: i64) -> Result<(), CapitalError> {
    if value <= 0 {
        return Err(CapitalError::SubscriptionError(
            "trial end timestamp must be positive".to_string(),
        ));
    }
    Ok(())
}

fn validate_ascii_identifier(
    label: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), CapitalError> {
    if value.is_empty()
        || value.len() > max_bytes
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(CapitalError::SubscriptionError(format!(
            "{label} must contain 1 to {max_bytes} ASCII letters, digits, `.`, `_`, or `-`"
        )));
    }
    Ok(())
}

fn trial_overflow_error() -> CapitalError {
    CapitalError::SubscriptionError("trial extension timestamp overflowed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupon_and_trial_values_are_bounded_and_redacted() {
        let coupon = CouponCode::try_new("BLACK_FRIDAY-25").expect("valid coupon");
        assert_eq!(coupon.as_str(), "BLACK_FRIDAY-25");
        assert!(!format!("{coupon:?}").contains("BLACK_FRIDAY"));
        assert!(CouponCode::try_new("").is_err());
        assert!(CouponCode::try_new("../coupon").is_err());

        let extension = TrialExtension::from_days_at(15, 1_000).expect("valid extension");
        assert_eq!(extension.days(), 15);
        assert_eq!(extension.ends_at(), 1_297_000);
        assert!(TrialExtension::from_days_at(0, 1_000).is_err());
        assert!(TrialExtension::from_days_at(731, 1_000).is_err());
        assert!(TrialExtension::from_days_at(1, -1).is_err());
        assert!(TrialExtension::from_days_at(1, i64::MAX).is_err());
    }
}
