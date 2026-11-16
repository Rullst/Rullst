//! How the buyer paid a one-off checkout, as the provider reports it.

use crate::error::CapitalError;

const MAX_METHOD_NAME_LEN: usize = 64;

/// How the buyer paid, as the provider reported it when confirming a
/// [`OneOffPayment`](crate::OneOffPayment). Since 13.0.0-alpha.6.
///
/// It is informational: whether the payment counts depends only on
/// [`OneOffPayment::is_paid`](crate::OneOffPayment::is_paid). Build it from
/// the provider's own name with [`Self::from_provider_name`]. A later 13.x
/// release may give a name that is [`Other`](Self::Other) today its own
/// variant, so match `Other` names as a fallback, not as the only branch for
/// a method you rely on.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PaymentMethod {
    /// Pix, Brazil's instant payment (InfinitePay's `pix`).
    Pix,
    /// A credit card (InfinitePay's `credit_card`).
    CreditCard,
    /// Another method, under the provider's own name.
    Other(PaymentMethodName),
}

impl PaymentMethod {
    /// Maps the provider's name for a method: `pix` is [`Self::Pix`],
    /// `credit_card` is [`Self::CreditCard`] and any other name of 1 to 64
    /// ASCII letters, digits, `_` or `-` is [`Self::Other`], stored in
    /// lowercase. Other names fail with [`CapitalError::PayloadParseError`].
    pub fn from_provider_name(name: &str) -> Result<Self, CapitalError> {
        let name = PaymentMethodName::parse(name)?;
        Ok(match name.as_str() {
            "pix" => Self::Pix,
            "credit_card" => Self::CreditCard,
            _ => Self::Other(name),
        })
    }

    /// The method's lowercase name: `pix`, `credit_card` or the provider's
    /// name of an [`Other`](Self::Other) method.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pix => "pix",
            Self::CreditCard => "credit_card",
            Self::Other(name) => name.as_str(),
        }
    }
}

impl std::fmt::Display for PaymentMethod {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A provider's name for a [`PaymentMethod::Other`] method: 1 to 64
/// lowercase ASCII letters, digits, `_` or `-`, never the name of a variant
/// Capital models. Since 13.0.0-alpha.6.
///
/// It is built only by [`PaymentMethod::from_provider_name`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PaymentMethodName(String);

impl PaymentMethodName {
    fn parse(name: &str) -> Result<Self, CapitalError> {
        let valid = !name.is_empty()
            && name.len() <= MAX_METHOD_NAME_LEN
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
        if !valid {
            return Err(CapitalError::PayloadParseError(
                "payment method must be 1 to 64 ASCII letters, digits, '_' or '-'".into(),
            ));
        }
        Ok(Self(name.to_ascii_lowercase()))
    }

    /// The lowercase name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
