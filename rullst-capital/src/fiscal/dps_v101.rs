//! Strict minimum DPS 1.01 builder for ordinary taxable domestic services.
//!
//! This type deliberately models a bounded subset. Construction guarantees the local structural
//! rules below; municipal parameters and SEFIN business rules still require official validation.

use chrono::{DateTime, NaiveDate, Utc};

use crate::fiscal::client::NfseEnvironment;
use crate::fiscal::contract::{MAX_DPS_XML_BYTES, NFSE_NAMESPACE};
use crate::fiscal::dps::escape_xml;
use crate::fiscal::models::{FiscalCustomer, FiscalEmitter, FiscalError, TaxRegime};

const MAX_MONEY_CENTS: u64 = 99_999_999_999_999_999;

/// ISSQN treatment declared for the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssTaxation {
    /// Ordinary taxable operation.
    Taxable,
    /// Constitutional immunity.
    Immune,
    /// Exported service.
    Export,
    /// Operation outside ISSQN incidence.
    NotTaxable,
}

impl IssTaxation {
    fn official_code(self) -> u8 {
        match self {
            Self::Taxable => 1,
            Self::Immune => 2,
            Self::Export => 3,
            Self::NotTaxable => 4,
        }
    }
}

/// Party responsible for retaining ISSQN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssRetention {
    /// ISSQN is not retained.
    NotRetained,
    /// The service customer retains ISSQN.
    Customer,
    /// A service intermediary retains ISSQN.
    Intermediary,
}

impl IssRetention {
    fn official_code(self) -> u8 {
        match self {
            Self::NotRetained => 1,
            Self::Customer => 2,
            Self::Intermediary => 3,
        }
    }
}

/// Strict inputs for the bounded DPS 1.01 builder.
#[derive(Debug, Clone)]
pub struct NfseDpsV101 {
    /// Exact 45-character official DPS identifier (`DPS` plus 42 digits).
    pub id: String,
    /// Numeric DPS series, one to five digits.
    pub series: String,
    /// Positive sequential DPS number.
    pub number: u64,
    /// UTC issuance timestamp.
    pub issued_at: DateTime<Utc>,
    /// Service competence date.
    pub competence_date: NaiveDate,
    /// Six-digit national service-tax code, without punctuation.
    pub service_code: String,
    /// Complete service description.
    pub description: String,
    /// Gross service amount in BRL cents.
    pub amount_cents: u64,
    /// Optional ISSQN rate in basis points (`200` means `2.00%`).
    pub iss_rate_basis_points: Option<u16>,
    /// Declared ISSQN treatment.
    pub iss_taxation: IssTaxation,
    /// Declared ISSQN retention.
    pub iss_retention: IssRetention,
    /// Seven-digit IBGE code of the service location.
    pub service_city_ibge: String,
}

impl NfseDpsV101 {
    /// Checks the schema-level subset and identity checks known locally.
    pub fn validate(
        &self,
        emitter: &FiscalEmitter,
        customer: &FiscalCustomer,
    ) -> Result<(), FiscalError> {
        validate_exact_digits("emitter.cnpj", &emitter.clean_cnpj(), 14)?;
        if !is_valid_cnpj(&emitter.clean_cnpj()) {
            return invalid("emitter.cnpj", "CNPJ check digits are invalid");
        }
        validate_exact_digits("emitter.ibge_code", &emitter.ibge_code, 7)?;
        validate_text("emitter.legal_name", &emitter.legal_name, 1, 150)?;
        validate_text(
            "emitter.inscricao_municipal",
            &emitter.inscricao_municipal,
            1,
            15,
        )?;

        let customer_document = customer.clean_doc();
        match customer_document.len() {
            11 if is_valid_cpf(&customer_document) => {}
            14 if is_valid_cnpj(&customer_document) => {}
            11 => return invalid("customer.doc_number", "CPF check digits are invalid"),
            14 => return invalid("customer.doc_number", "CNPJ check digits are invalid"),
            _ => {
                return invalid(
                    "customer.doc_number",
                    "expected an 11-digit CPF or 14-digit CNPJ",
                );
            }
        }
        validate_text("customer.name", &customer.name, 1, 150)?;
        validate_text("customer.email", &customer.email, 3, 80)?;
        if !customer.email.contains('@') {
            return invalid("customer.email", "email must contain @");
        }

        if self.id.len() != 45
            || !self.id.starts_with("DPS")
            || !self.id[3..].bytes().all(|byte| byte.is_ascii_digit())
        {
            return invalid("dps.id", "expected DPS followed by exactly 42 digits");
        }
        validate_digits_range("dps.series", &self.series, 1, 5)?;
        if self.number == 0 || self.number > 999_999_999_999_999 {
            return invalid(
                "dps.number",
                "expected a positive number of at most 15 digits",
            );
        }
        let expected_id = format!(
            "DPS{}2{}{:0>5}{:0>15}",
            emitter.ibge_code,
            emitter.clean_cnpj(),
            self.series,
            self.number
        );
        if self.id != expected_id {
            return invalid(
                "dps.id",
                "identifier does not match the emitter municipality, CNPJ, series and number",
            );
        }
        validate_exact_digits("dps.service_code", &self.service_code, 6)?;
        validate_text("dps.description", &self.description, 1, 2_000)?;
        validate_exact_digits("dps.service_city_ibge", &self.service_city_ibge, 7)?;
        if self.amount_cents > MAX_MONEY_CENTS {
            return invalid("dps.amount_cents", "amount exceeds the DPS decimal limit");
        }
        if self
            .iss_rate_basis_points
            .is_some_and(|basis_points| basis_points > 999)
        {
            return invalid(
                "dps.iss_rate_basis_points",
                "ISSQN rate must be between 0.00% and 9.99%",
            );
        }
        Ok(())
    }
}

/// Builds an unsigned DPS 1.01 XML for the supported domestic-service subset.
pub fn build_dps_xml_v1_01(
    emitter: &FiscalEmitter,
    customer: &FiscalCustomer,
    dps: &NfseDpsV101,
    environment: NfseEnvironment,
) -> Result<String, FiscalError> {
    dps.validate(emitter, customer)?;

    let customer_document = customer.clean_doc();
    let customer_tag = if customer_document.len() == 14 {
        "CNPJ"
    } else {
        "CPF"
    };
    let environment_code = match environment {
        NfseEnvironment::Production => 1,
        NfseEnvironment::Mock | NfseEnvironment::Homologation => 2,
    };
    let (simple_status, simple_assessment) = tax_regime_codes(emitter.tax_regime);
    let assessment_xml = simple_assessment
        .map(|code| format!("<regApTribSN>{code}</regApTribSN>"))
        .unwrap_or_default();
    let rate_xml = dps
        .iss_rate_basis_points
        .map(|basis_points| format!("<pAliq>{}</pAliq>", format_rate(basis_points)))
        .unwrap_or_default();

    let xml = format!(
        "<DPS xmlns=\"{NFSE_NAMESPACE}\" versao=\"1.01\"><infDPS Id=\"{id}\"><tpAmb>{environment_code}</tpAmb><dhEmi>{issued_at}</dhEmi><verAplic>Rullst-12.0</verAplic><serie>{series}</serie><nDPS>{number}</nDPS><dCompet>{competence}</dCompet><tpEmit>1</tpEmit><cLocEmi>{emitter_city}</cLocEmi><prest><CNPJ>{emitter_document}</CNPJ><IM>{municipal_registration}</IM><xNome>{emitter_name}</xNome><regTrib><opSimpNac>{simple_status}</opSimpNac>{assessment_xml}<regEspTrib>0</regEspTrib></regTrib></prest><toma><{customer_tag}>{customer_document}</{customer_tag}><xNome>{customer_name}</xNome><email>{customer_email}</email></toma><serv><locPrest><cLocPrestacao>{service_city}</cLocPrestacao></locPrest><cServ><cTribNac>{service_code}</cTribNac><xDescServ>{description}</xDescServ></cServ></serv><valores><vServPrest><vServ>{amount}</vServ></vServPrest><trib><tribMun><tribISSQN>{taxation}</tribISSQN><tpRetISSQN>{retention}</tpRetISSQN>{rate_xml}</tribMun><totTrib><indTotTrib>0</indTotTrib></totTrib></trib></valores></infDPS></DPS>",
        id = dps.id,
        issued_at = dps.issued_at.format("%Y-%m-%dT%H:%M:%S+00:00"),
        series = dps.series,
        number = dps.number,
        competence = dps.competence_date.format("%Y-%m-%d"),
        emitter_city = emitter.ibge_code,
        emitter_document = emitter.clean_cnpj(),
        municipal_registration = escape_xml(emitter.inscricao_municipal.trim()),
        emitter_name = escape_xml(emitter.legal_name.trim()),
        customer_document = customer_document,
        customer_name = escape_xml(customer.name.trim()),
        customer_email = escape_xml(customer.email.trim()),
        service_city = dps.service_city_ibge,
        service_code = dps.service_code,
        description = escape_xml(dps.description.trim()),
        amount = format_money(dps.amount_cents),
        taxation = dps.iss_taxation.official_code(),
        retention = dps.iss_retention.official_code(),
    );
    if xml.len() > MAX_DPS_XML_BYTES {
        return invalid("dps.xml", "generated document exceeds the one MiB limit");
    }
    Ok(xml)
}

fn tax_regime_codes(regime: TaxRegime) -> (u8, Option<u8>) {
    match regime {
        TaxRegime::RegimeNormal => (1, None),
        TaxRegime::SimplesNacional => (3, Some(1)),
        TaxRegime::SimplesNacionalExcesso => (3, Some(2)),
    }
}

fn format_money(cents: u64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

fn format_rate(basis_points: u16) -> String {
    format!("{}.{:02}", basis_points / 100, basis_points % 100)
}

fn validate_exact_digits(
    field: &'static str,
    value: &str,
    length: usize,
) -> Result<(), FiscalError> {
    if value.len() != length || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return invalid(field, format!("expected exactly {length} digits"));
    }
    Ok(())
}

fn validate_digits_range(
    field: &'static str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), FiscalError> {
    if !(minimum..=maximum).contains(&value.len())
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return invalid(
            field,
            format!("expected between {minimum} and {maximum} digits"),
        );
    }
    Ok(())
}

fn validate_text(
    field: &'static str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<(), FiscalError> {
    let trimmed = value.trim();
    if !(minimum..=maximum).contains(&trimmed.chars().count()) {
        return invalid(
            field,
            format!("expected between {minimum} and {maximum} characters"),
        );
    }
    if !trimmed.chars().all(is_xml_1_0_character) {
        return invalid(field, "contains a character forbidden by XML 1.0");
    }
    Ok(())
}

fn is_xml_1_0_character(character: char) -> bool {
    matches!(character, '\u{9}' | '\u{A}' | '\u{D}')
        || ('\u{20}'..='\u{D7FF}').contains(&character)
        || ('\u{E000}'..='\u{FFFD}').contains(&character)
        || ('\u{10000}'..='\u{10FFFF}').contains(&character)
}

fn is_valid_cpf(value: &str) -> bool {
    let digits = value.bytes().map(|byte| byte - b'0').collect::<Vec<_>>();
    if digits.windows(2).all(|pair| pair[0] == pair[1]) {
        return false;
    }
    cpf_digit(&digits[..9], 10) == digits[9] && cpf_digit(&digits[..10], 11) == digits[10]
}

fn cpf_digit(digits: &[u8], initial_weight: u32) -> u8 {
    let sum = digits
        .iter()
        .enumerate()
        .map(|(index, digit)| u32::from(*digit) * (initial_weight - index as u32))
        .sum::<u32>();
    let remainder = (sum * 10) % 11;
    if remainder == 10 { 0 } else { remainder as u8 }
}

fn is_valid_cnpj(value: &str) -> bool {
    let digits = value.bytes().map(|byte| byte - b'0').collect::<Vec<_>>();
    if digits.windows(2).all(|pair| pair[0] == pair[1]) {
        return false;
    }
    cnpj_digit(&digits[..12], &[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]) == digits[12]
        && cnpj_digit(&digits[..13], &[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]) == digits[13]
}

fn cnpj_digit(digits: &[u8], weights: &[u32]) -> u8 {
    let remainder = digits
        .iter()
        .zip(weights)
        .map(|(digit, weight)| u32::from(*digit) * weight)
        .sum::<u32>()
        % 11;
    if remainder < 2 {
        0
    } else {
        (11 - remainder) as u8
    }
}

fn invalid<T>(field: &'static str, reason: impl Into<String>) -> Result<T, FiscalError> {
    Err(FiscalError::InvalidInput {
        field,
        reason: reason.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiscal::models::TaxRegime;

    fn emitter() -> FiscalEmitter {
        FiscalEmitter {
            cnpj: "11.222.333/0001-81".to_string(),
            inscricao_municipal: "12345".to_string(),
            legal_name: "Rullst Serviços Ltda".to_string(),
            trade_name: None,
            ibge_code: "3550308".to_string(),
            tax_regime: TaxRegime::SimplesNacional,
        }
    }

    fn customer() -> FiscalCustomer {
        FiscalCustomer {
            doc_number: "529.982.247-25".to_string(),
            name: "Cliente & Filhos".to_string(),
            email: "fiscal@example.com".to_string(),
            zip_code: None,
            address: None,
            ibge_code: None,
        }
    }

    fn dps() -> NfseDpsV101 {
        NfseDpsV101 {
            id: "DPS355030821122233300018100001000000000000101".to_string(),
            series: "1".to_string(),
            number: 101,
            issued_at: DateTime::from_timestamp(1_767_268_800, 0).unwrap(),
            competence_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            service_code: "010301".to_string(),
            description: "Processamento de dados & SaaS".to_string(),
            amount_cents: 12_345,
            iss_rate_basis_points: Some(200),
            iss_taxation: IssTaxation::Taxable,
            iss_retention: IssRetention::NotRetained,
            service_city_ibge: "3550308".to_string(),
        }
    }

    #[test]
    fn builds_current_minimum_without_float_money_or_retention_confusion() {
        let xml = build_dps_xml_v1_01(
            &emitter(),
            &customer(),
            &dps(),
            NfseEnvironment::Homologation,
        )
        .unwrap();

        assert!(xml.contains("versao=\"1.01\""));
        assert!(xml.contains("<tpAmb>2</tpAmb>"));
        assert!(xml.contains("<locPrest><cLocPrestacao>3550308"));
        assert!(xml.contains("<cTribNac>010301</cTribNac>"));
        assert!(xml.contains("<vServ>123.45</vServ>"));
        assert!(xml.contains("<tribISSQN>1</tribISSQN><tpRetISSQN>1</tpRetISSQN>"));
        assert!(xml.contains("<totTrib><indTotTrib>0</indTotTrib></totTrib>"));
        assert!(xml.contains("Cliente &amp; Filhos"));
        assert!(!xml.contains("<Signature"));
    }

    #[test]
    fn rejects_invalid_identity_money_and_xml_characters() {
        let mut invalid_dps = dps();
        invalid_dps.service_code = "1.03.01".to_string();
        assert!(invalid_dps.validate(&emitter(), &customer()).is_err());

        invalid_dps = dps();
        invalid_dps.description = "bad\u{0}text".to_string();
        assert!(invalid_dps.validate(&emitter(), &customer()).is_err());

        invalid_dps = dps();
        invalid_dps.number = 102;
        assert!(invalid_dps.validate(&emitter(), &customer()).is_err());

        let mut invalid_customer = customer();
        invalid_customer.doc_number = "52998224724".to_string();
        assert!(dps().validate(&emitter(), &invalid_customer).is_err());
    }
}
