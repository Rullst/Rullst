use super::*;
use crate::fiscal::models::TaxRegime;
use chrono::DateTime;

fn emitter(tax_regime: TaxRegime) -> FiscalEmitter {
    FiscalEmitter {
        cnpj: "12.345.678/0001-90".to_string(),
        inscricao_municipal: "1234567".to_string(),
        legal_name: "Rullst SaaS".to_string(),
        trade_name: None,
        ibge_code: "3550308".to_string(),
        tax_regime,
    }
}

fn customer() -> FiscalCustomer {
    FiscalCustomer {
        doc_number: "529.982.247-25".to_string(),
        name: "Cliente".to_string(),
        email: "cliente@example.com".to_string(),
        zip_code: None,
        address: None,
        ibge_code: None,
    }
}

fn dps() -> NfseDps {
    NfseDps {
        id: String::new(),
        series: "1".to_string(),
        number: 101,
        issued_at: DateTime::from_timestamp(1_767_268_800, 0).expect("timestamp"),
        service_code: "1.03.01".to_string(),
        description: "Plano".to_string(),
        amount: 99.0,
        iss_rate: 2.0,
        iss_retained: false,
        service_city_ibge: "3550308".to_string(),
    }
}

#[test]
fn every_interpolated_value_is_escaped() {
    let mut injected = dps();
    injected.service_code =
        "1.03.01</cTribNac><xDescServ>Forged</xDescServ></cServ><cServ><cTribNac>x".to_string();
    injected.service_city_ibge = "3550308</cLocIncid><x>".to_string();
    injected.series = "1<a/>".to_string();
    let mut emitter = emitter(TaxRegime::SimplesNacional);
    emitter.inscricao_municipal = "12\"<IM>".to_string();
    let xml = build_dps_xml(&emitter, &customer(), &injected);
    assert!(!xml.contains("<xDescServ>Forged"));
    assert!(xml.contains("&lt;/cTribNac&gt;&lt;xDescServ&gt;Forged"));
    assert!(!xml.contains("<x>") && !xml.contains("<a/>"));
    assert!(xml.contains("<IM>12&quot;&lt;IM&gt;</IM>"));
    assert_eq!(xml.matches("<cServ>").count(), 1);

    injected.id = "DPS1\" injected=\"1".to_string();
    let xml = build_dps_xml(&emitter, &customer(), &injected);
    assert!(xml.contains("Id=\"DPS1&quot; injected=&quot;1\""));
}

#[test]
fn generated_id_uses_the_official_zero_padded_layout() {
    let xml = build_dps_xml(&emitter(TaxRegime::SimplesNacional), &customer(), &dps());
    let id = "DPS355030821234567800019000001000000000000101";
    assert_eq!(id.len(), 45);
    assert!(xml.contains(&format!("<infDPS Id=\"{id}\">")));
}

#[test]
fn preview_declares_homologation_utc_offset_and_official_tax_codes() {
    let xml = build_dps_xml(&emitter(TaxRegime::SimplesNacional), &customer(), &dps());
    assert!(xml.contains("<tpAmb>2</tpAmb>"));
    assert!(xml.contains(&format!("<verAplic>{APPLICATION_VERSION}</verAplic>")));
    assert!(!xml.contains("Rullst-12.0"));
    assert!(xml.contains("<dhEmi>2026-01-01T12:00:00+00:00</dhEmi>"));
    assert!(xml.contains("<opSimpNac>3</opSimpNac><regApTribSN>1</regApTribSN>"));
    assert!(xml.contains("<tribISSQN>1</tribISSQN><tpRetISSQN>1</tpRetISSQN>"));

    let normal = build_dps_xml(&emitter(TaxRegime::RegimeNormal), &customer(), &dps());
    assert!(normal.contains("<opSimpNac>1</opSimpNac><regEspTrib>"));
    let excess = build_dps_xml(
        &emitter(TaxRegime::SimplesNacionalExcesso),
        &customer(),
        &dps(),
    );
    assert!(excess.contains("<opSimpNac>3</opSimpNac><regApTribSN>2</regApTribSN>"));

    let mut retained = dps();
    retained.iss_retained = true;
    let xml = build_dps_xml(&emitter(TaxRegime::SimplesNacional), &customer(), &retained);
    assert!(xml.contains("<tribISSQN>1</tribISSQN><tpRetISSQN>2</tpRetISSQN>"));
}
