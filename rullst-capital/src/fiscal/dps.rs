use crate::fiscal::models::{FiscalCustomer, FiscalEmitter, NfseDps};

/// Generates a standardized XML document for the Declaração de Prestação de Serviços (DPS).
pub fn build_dps_xml(emitter: &FiscalEmitter, customer: &FiscalCustomer, dps: &NfseDps) -> String {
    let clean_cnpj = emitter.clean_cnpj();
    let clean_doc = customer.clean_doc();
    let doc_tag = if customer.is_company() { "CNPJ" } else { "CPF" };

    let date_str = dps.issued_at.format("%Y-%m-%d").to_string();
    let time_str = dps.issued_at.format("%H:%M:%S").to_string();

    let dps_id = if dps.id.starts_with("DPS") {
        dps.id.clone()
    } else {
        format!(
            "DPS{}{:>03}{:>015}",
            emitter.ibge_code, dps.series, dps.number
        )
    };

    let iss_retido_int = if dps.iss_retained { 1 } else { 2 };
    let regime_int = emitter.tax_regime as u8;

    format!(
        r#"<DPS xmlns="http://www.sped.fazenda.gov.br/nfse" versao="1.00"><infDPS Id="{dps_id}"><tpAmb>1</tpAmb><dhEmi>{date_str}T{time_str}Z</dhEmi><verAplic>Rullst-12.0</verAplic><serie>{serie}</serie><nDPS>{ndps}</nDPS><dCompet>{date_str}</dCompet><tpEmit>1</tpEmit><cLocEmi>{ibge}</cLocEmi><prest><CNPJ>{cnpj}</CNPJ><IM>{im}</IM><xNome>{xnome}</xNome><regTrib><opSimpNac>{regime}</opSimpNac><regEspTrib>0</regEspTrib></regTrib></prest><toma><{doc_tag}>{doc_val}</{doc_tag}><xNome>{cust_name}</xNome><email>{cust_email}</email></toma><serv><cServ><cTribNac>{serv_code}</cTribNac><xDescServ>{serv_desc}</xDescServ></cServ></serv><valores><vServPrest><vServ>{vserv:.2}</vServ></vServPrest><trib><tribMun><tribISSQN>{iss_ret}</tribISSQN><cLocIncid>{serv_city}</cLocIncid><pAliq>{aliq:.2}</pAliq></tribMun></trib></valores></infDPS></DPS>"#,
        dps_id = dps_id,
        date_str = date_str,
        time_str = time_str,
        serie = dps.series,
        ndps = dps.number,
        ibge = emitter.ibge_code,
        cnpj = clean_cnpj,
        im = emitter.inscricao_municipal,
        xnome = escape_xml(&emitter.legal_name),
        regime = regime_int,
        doc_tag = doc_tag,
        doc_val = clean_doc,
        cust_name = escape_xml(&customer.name),
        cust_email = escape_xml(&customer.email),
        serv_code = dps.service_code,
        serv_desc = escape_xml(&dps.description),
        vserv = dps.amount,
        iss_ret = iss_retido_int,
        serv_city = dps.service_city_ibge,
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
