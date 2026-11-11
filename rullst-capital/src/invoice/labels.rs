//! Localized labels of the invoice HTML and PDF.

use serde::{Deserialize, Serialize};

const MAX_LABEL_CHARS: usize = 64;

/// A built-in language for the invoice labels. Since 13.0.0-alpha.5.
///
/// Portuguese and Spanish use "Recibo" (receipt): Capital does not issue tax
/// invoices (nota fiscal, factura), so the document must not read as one.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InvoiceLocale {
    /// English (`en`): "Invoice", "Billed To", "Date", "Description",
    /// "Amount", "Total". The default, unchanged from 13.0.0-alpha.4.
    #[default]
    En,
    /// Brazilian Portuguese (`pt-BR`): "Recibo", "Recebido de", "Data",
    /// "Descrição", "Valor", "Total".
    PtBr,
    /// Spanish (`es`): "Recibo", "Recibido de", "Fecha", "Descripción",
    /// "Importe", "Total".
    Es,
}

impl InvoiceLocale {
    /// The built-in locale of a language tag such as `pt-BR`, `pt_br`, `es-MX`
    /// or `en-US` (any Portuguese tag maps to `PtBr`, any Spanish one to
    /// `Es`); `None` for another language.
    pub fn from_tag(tag: &str) -> Option<Self> {
        let language = tag
            .trim()
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match language.as_str() {
            "en" => Some(Self::En),
            "pt" => Some(Self::PtBr),
            "es" => Some(Self::Es),
            _ => None,
        }
    }

    /// The BCP 47 tag written to `<html lang>`.
    pub fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::PtBr => "pt-BR",
            Self::Es => "es",
        }
    }

    /// The labels of this locale.
    pub fn labels(self) -> InvoiceLabels {
        let [title, billed_to, date, description, amount, total] = match self {
            Self::En => [
                "Invoice",
                "Billed To",
                "Date",
                "Description",
                "Amount",
                "Total",
            ],
            Self::PtBr => [
                "Recibo",
                "Recebido de",
                "Data",
                "Descrição",
                "Valor",
                "Total",
            ],
            Self::Es => [
                "Recibo",
                "Recibido de",
                "Fecha",
                "Descripción",
                "Importe",
                "Total",
            ],
        };
        InvoiceLabels {
            language: self.tag().to_string(),
            title: title.to_string(),
            billed_to: billed_to.to_string(),
            date: date.to_string(),
            description: description.to_string(),
            amount: amount.to_string(),
            total: total.to_string(),
        }
    }
}

/// The words the invoice HTML and PDF print, and the document language.
/// Since 13.0.0-alpha.5.
///
/// Start from [`InvoiceLocale::labels`] and change fields for another
/// language or wording, for example `labels.title = "Receipt".into()`. Each
/// label holds 1 to 64 characters without control characters; `language` is a
/// BCP 47 tag of ASCII letters, digits and `-`. The default is English.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLabels {
    /// Language tag of the HTML document (`<html lang>`).
    pub language: String,
    /// Document title, followed by the invoice ID ("Invoice", "Recibo").
    pub title: String,
    /// Label of the customer e-mail address.
    pub billed_to: String,
    /// Label of the issue date.
    pub date: String,
    /// Heading of the item description column.
    pub description: String,
    /// Heading of the item amount column.
    pub amount: String,
    /// Label of the total.
    pub total: String,
}

impl Default for InvoiceLabels {
    fn default() -> Self {
        InvoiceLocale::En.labels()
    }
}

impl InvoiceLabels {
    pub(crate) fn validate(&self) -> Result<(), crate::error::CapitalError> {
        let language_valid = (1..=35).contains(&self.language.len())
            && self
                .language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
        if !language_valid {
            return Err(super::invalid_invoice(
                "invoice language must be a BCP 47 tag of 1 to 35 ASCII letters, digits and '-'",
            ));
        }
        for label in [
            &self.title,
            &self.billed_to,
            &self.date,
            &self.description,
            &self.amount,
            &self.total,
        ] {
            let characters = label.chars().count();
            if characters == 0
                || characters > MAX_LABEL_CHARS
                || label.trim() != label
                || label.chars().any(char::is_control)
            {
                return Err(super::invalid_invoice(
                    "invoice labels must contain 1 to 64 characters without control characters or surrounding whitespace",
                ));
            }
        }
        Ok(())
    }
}
