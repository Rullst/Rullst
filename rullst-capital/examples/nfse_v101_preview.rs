//! Emits an unsigned, non-authorized DPS 1.01 fixture for offline inspection.

use chrono::{DateTime, NaiveDate};
use rullst_capital::fiscal::{
    FiscalCustomer, FiscalEmitter, IssRetention, IssTaxation, NFSE_PRODUCTION_V1_01_20260209,
    NfseDpsSchemaValidator, NfseDpsV101, NfseEnvironment, TaxRegime, build_dps_xml_v1_01,
};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let emitter = FiscalEmitter {
        cnpj: "11.222.333/0001-81".to_string(),
        inscricao_municipal: "12345".to_string(),
        legal_name: "Rullst Serviços Ltda".to_string(),
        trade_name: None,
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::SimplesNacional,
    };
    let customer = FiscalCustomer {
        doc_number: "529.982.247-25".to_string(),
        name: "Cliente Exemplo".to_string(),
        email: "fiscal@example.com".to_string(),
        zip_code: None,
        address: None,
        ibge_code: None,
    };
    let dps = NfseDpsV101 {
        id: "DPS355030821122233300018100001000000000000101".to_string(),
        series: "1".to_string(),
        number: 101,
        issued_at: DateTime::from_timestamp(1_767_268_800, 0)
            .ok_or("example timestamp is outside chrono's supported range")?,
        competence_date: NaiveDate::from_ymd_opt(2026, 1, 1)
            .ok_or("example date is outside chrono's supported range")?,
        service_code: "010301".to_string(),
        description: "Processamento de dados e SaaS".to_string(),
        amount_cents: 12_345,
        iss_rate_basis_points: Some(200),
        iss_taxation: IssTaxation::Taxable,
        iss_retention: IssRetention::NotRetained,
        service_city_ibge: "3550308".to_string(),
    };

    let xml = build_dps_xml_v1_01(&emitter, &customer, &dps, NfseEnvironment::Homologation)?;
    if let Ok(schema_directory) = std::env::var("RULLST_NFSE_XSD_DIR") {
        let validator = NfseDpsSchemaValidator::from_pinned_directory(
            schema_directory,
            &NFSE_PRODUCTION_V1_01_20260209,
        )?;
        validator.validate(&xml)?;
    }
    writeln!(std::io::stdout().lock(), "{xml}")?;
    Ok(())
}
