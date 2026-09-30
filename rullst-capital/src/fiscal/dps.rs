use crate::fiscal::dps_v101::{APPLICATION_VERSION, tax_regime_codes};
use crate::fiscal::models::{FiscalCustomer, FiscalEmitter, NfseDps};

/// Generates the legacy floating-point DPS preview.
///
/// Compatibility-only: use [`crate::fiscal::build_dps_xml_v1_01`] for a
/// validated document. Every interpolated value is XML-escaped. Because this
/// path never transmits, the preview always declares homologation
/// (`tpAmb` 2); `dhEmi` carries an explicit `+00:00` offset, `opSimpNac` and
/// `regApTribSN` use the official regime codes, and ISSQN is declared as an
/// ordinary taxable operation (`tribISSQN` 1) with `tpRetISSQN` from the
/// retention flag. A non-`DPS` ID is built in the official 45-character
/// layout from the municipality, CNPJ, zero-padded series and number.
pub fn build_dps_xml(emitter: &FiscalEmitter, customer: &FiscalCustomer, dps: &NfseDps) -> String {
    let clean_cnpj = emitter.clean_cnpj();
    let clean_doc = customer.clean_doc();
    let doc_tag = if customer.is_company() { "CNPJ" } else { "CPF" };

    let issued_at = dps.issued_at.format("%Y-%m-%dT%H:%M:%S+00:00").to_string();
    let date_str = dps.issued_at.format("%Y-%m-%d").to_string();

    let dps_id = if dps.id.starts_with("DPS") {
        dps.id.clone()
    } else {
        format!(
            "DPS{}2{}{:0>5}{:0>15}",
            emitter.ibge_code, clean_cnpj, dps.series, dps.number
        )
    };

    let (simple_status, simple_assessment) = tax_regime_codes(emitter.tax_regime);
    let assessment_xml = simple_assessment
        .map(|code| format!("<regApTribSN>{code}</regApTribSN>"))
        .unwrap_or_default();
    // Official tpRetISSQN: 1 = not retained, 2 = retained by the customer.
    let retention = if dps.iss_retained { 2 } else { 1 };

    format!(
        r#"<DPS xmlns="http://www.sped.fazenda.gov.br/nfse" versao="1.00"><infDPS Id="{dps_id}"><tpAmb>2</tpAmb><dhEmi>{issued_at}</dhEmi><verAplic>{APPLICATION_VERSION}</verAplic><serie>{serie}</serie><nDPS>{ndps}</nDPS><dCompet>{date_str}</dCompet><tpEmit>1</tpEmit><cLocEmi>{ibge}</cLocEmi><prest><CNPJ>{cnpj}</CNPJ><IM>{im}</IM><xNome>{xnome}</xNome><regTrib><opSimpNac>{simple_status}</opSimpNac>{assessment_xml}<regEspTrib>0</regEspTrib></regTrib></prest><toma><{doc_tag}>{doc_val}</{doc_tag}><xNome>{cust_name}</xNome><email>{cust_email}</email></toma><serv><cServ><cTribNac>{serv_code}</cTribNac><xDescServ>{serv_desc}</xDescServ></cServ></serv><valores><vServPrest><vServ>{vserv:.2}</vServ></vServPrest><trib><tribMun><tribISSQN>1</tribISSQN><tpRetISSQN>{retention}</tpRetISSQN><cLocIncid>{serv_city}</cLocIncid><pAliq>{aliq:.2}</pAliq></tribMun></trib></valores></infDPS></DPS>"#,
        dps_id = escape_xml(&dps_id),
        issued_at = issued_at,
        date_str = date_str,
        serie = escape_xml(&dps.series),
        ndps = dps.number,
        ibge = escape_xml(&emitter.ibge_code),
        cnpj = clean_cnpj,
        im = escape_xml(&emitter.inscricao_municipal),
        xnome = escape_xml(&emitter.legal_name),
        simple_status = simple_status,
        assessment_xml = assessment_xml,
        doc_tag = doc_tag,
        doc_val = clean_doc,
        cust_name = escape_xml(&customer.name),
        cust_email = escape_xml(&customer.email),
        serv_code = escape_xml(&dps.service_code),
        serv_desc = escape_xml(&dps.description),
        vserv = dps.amount,
        retention = retention,
        serv_city = escape_xml(&dps.service_city_ibge),
        aliq = dps.iss_rate,
    )
}

/// Escapes XML special characters according to XML 1.0 specifications.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
#[path = "dps_tests.rs"]
mod tests;
