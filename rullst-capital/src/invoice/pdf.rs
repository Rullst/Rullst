use super::{Invoice, format_money, invalid_invoice};
use crate::error::CapitalError;
use printpdf::{
    BuiltinFont, Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point,
    Pt, TextItem,
};

const MAX_FONT_BYTES: usize = 8 * 1024 * 1024;
const MAX_PDF_BYTES: usize = 16 * 1024 * 1024;
const DESCRIPTION_CHARS_PER_LINE: usize = 64;
const LINES_PER_PAGE: usize = 42;

impl Invoice {
    /// Renders a bounded A4 PDF with the embedded Helvetica WinAnsi subset.
    ///
    /// Portuguese/Western-European characters supported by WinAnsi are
    /// preserved. Use [`Self::generate_pdf_with_font`] for other scripts.
    pub fn generate_pdf(&self) -> Result<Vec<u8>, CapitalError> {
        let total_minor = self.validated_total()?;
        let lines = self.render_lines(total_minor);
        for line in &lines {
            if !line.chars().all(is_win_ansi) {
                return Err(invalid_invoice(
                    "default PDF font supports only WinAnsi; provide a font containing every invoice character",
                ));
            }
        }
        render_pdf(self, lines, None)
    }

    /// Renders a bounded A4 PDF using a caller-supplied TTF/OTF font.
    pub fn generate_pdf_with_font(&self, font_bytes: &[u8]) -> Result<Vec<u8>, CapitalError> {
        if font_bytes.is_empty() || font_bytes.len() > MAX_FONT_BYTES {
            return Err(invalid_invoice("PDF font must contain 1 byte to 8 MiB"));
        }
        let total_minor = self.validated_total()?;
        let lines = self.render_lines(total_minor);
        let mut warnings = Vec::new();
        let font = ParsedFont::from_bytes(font_bytes, 0, &mut warnings)
            .ok_or_else(|| invalid_invoice("PDF font could not be parsed"))?;
        for character in lines.iter().flat_map(|line| line.chars()) {
            if character != ' '
                && font
                    .lookup_glyph_index(character as u32)
                    .is_none_or(|glyph| glyph == 0)
            {
                return Err(invalid_invoice(format!(
                    "PDF font is missing required Unicode code point U+{:04X}",
                    character as u32
                )));
            }
        }
        render_pdf(self, lines, Some(font))
    }

    fn render_lines(&self, total_minor: u64) -> Vec<String> {
        let labels = &self.labels;
        let mut lines = vec![
            format!("{} {}", labels.title, self.invoice_id),
            format!("{}: {}", labels.billed_to, self.customer_email),
            format!("{}: {}", labels.date, self.date.format("%Y-%m-%d")),
            String::new(),
            format!("{} | {}", labels.description, labels.amount),
        ];
        let currency = self.currency.to_ascii_uppercase();
        for item in &self.items {
            let chunks = wrap_chars(&item.description, DESCRIPTION_CHARS_PER_LINE);
            for (index, chunk) in chunks.into_iter().enumerate() {
                if index == 0 {
                    lines.push(format!(
                        "{} | {}",
                        chunk,
                        format_money(item.amount_minor, &currency)
                    ));
                } else {
                    lines.push(format!("  {chunk}"));
                }
            }
        }
        lines.push(String::new());
        lines.push(format!(
            "{}: {}",
            labels.total,
            format_money(total_minor, &currency)
        ));
        lines
    }
}

fn render_pdf(
    invoice: &Invoice,
    lines: Vec<String>,
    external_font: Option<ParsedFont>,
) -> Result<Vec<u8>, CapitalError> {
    let mut document =
        PdfDocument::new(&format!("{} {}", invoice.labels.title, invoice.invoice_id));
    document.metadata.info.creator = "Rullst Capital".to_string();
    document.metadata.info.producer = "Rullst Capital".to_string();
    document.metadata.info.subject = "Bounded invoice document".to_string();

    let font = match external_font {
        Some(font) => {
            let font_id = document.add_font(&font);
            PdfFontHandle::External(font_id)
        }
        None => PdfFontHandle::Builtin(BuiltinFont::Helvetica),
    };
    let pages = lines
        .chunks(LINES_PER_PAGE)
        .map(|page_lines| page(page_lines, &font))
        .collect();
    document.pages = pages;

    let mut warnings = Vec::new();
    let bytes = document.save(&PdfSaveOptions::default(), &mut warnings);
    if bytes.len() > MAX_PDF_BYTES || !bytes.starts_with(b"%PDF-") {
        return Err(CapitalError::ProviderRequestFailed(
            "PDF renderer produced an invalid or oversized document".to_string(),
        ));
    }
    Ok(bytes)
}

fn page(lines: &[String], font: &PdfFontHandle) -> PdfPage {
    let mut operations = vec![
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point::new(Mm(18.0), Mm(280.0)),
        },
        Op::SetFont {
            font: font.clone(),
            size: Pt(10.0),
        },
        Op::SetLineHeight { lh: Pt(14.0) },
    ];
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            operations.push(Op::AddLineBreak);
        }
        operations.push(Op::ShowText {
            items: vec![TextItem::Text(line.clone())],
        });
    }
    operations.push(Op::EndTextSection);
    PdfPage::new(Mm(210.0), Mm(297.0), operations)
}

fn wrap_chars(value: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    for character in value.chars() {
        current.push(character);
        if current.chars().count() == width {
            result.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

fn is_win_ansi(character: char) -> bool {
    matches!(character as u32, 0x20..=0x7e | 0xa0..=0xff)
        || matches!(
            character,
            '\u{20ac}'
                | '\u{201a}'
                | '\u{0192}'
                | '\u{201e}'
                | '\u{2026}'
                | '\u{2020}'
                | '\u{2021}'
                | '\u{02c6}'
                | '\u{2030}'
                | '\u{0160}'
                | '\u{2039}'
                | '\u{0152}'
                | '\u{017d}'
                | '\u{2018}'
                | '\u{2019}'
                | '\u{201c}'
                | '\u{201d}'
                | '\u{2022}'
                | '\u{2013}'
                | '\u{2014}'
                | '\u{02dc}'
                | '\u{2122}'
                | '\u{0161}'
                | '\u{203a}'
                | '\u{0153}'
                | '\u{017e}'
                | '\u{0178}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::InvoiceItem;
    use chrono::Utc;
    use printpdf::PdfParseOptions;

    fn invoice(description: &str) -> Invoice {
        Invoice::new("INV-2026-1", "cliente@example.com", Utc::now(), "brl")
            .with_item(description, 4_990)
    }

    #[test]
    fn money_uses_the_currency_minor_unit_exponent() {
        assert_eq!(format_money(4_990, "BRL"), "49.90 BRL");
        assert_eq!(format_money(2_500, "JPY"), "2500 JPY");
        assert_eq!(format_money(12_340, "KWD"), "12.340 KWD");
        assert_eq!(format_money(7, "KWD"), "0.007 KWD");
    }

    fn pdf_text(bytes: &[u8]) -> String {
        PdfDocument::parse(bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse generated PDF")
            .extract_text()
            .into_iter()
            .flatten()
            .collect()
    }

    #[test]
    fn localized_labels_reach_the_pdf_and_its_title() {
        let english = pdf_text(&invoice("Plan").generate_pdf().expect("English PDF"));
        for label in [
            "Invoice INV-2026-1",
            "Billed To: ",
            "Date: ",
            "Description | Amount",
            "Total: 49.90 BRL",
        ] {
            assert!(english.contains(label), "{label}: {english:?}");
        }
        let receipt = invoice("Mensalidade")
            .with_locale(crate::InvoiceLocale::PtBr)
            .generate_pdf()
            .expect("WinAnsi covers Portuguese labels");
        let text = pdf_text(&receipt);
        for label in [
            "Recibo INV-2026-1",
            "Recebido de: ",
            "Data: ",
            "Descrição | Valor",
            "Total: 49.90 BRL",
        ] {
            assert!(text.contains(label), "{label}: {text:?}");
        }
        assert!(!text.contains("Invoice"), "{text:?}");
        let spanish = pdf_text(
            &invoice("Clase")
                .with_locale(crate::InvoiceLocale::Es)
                .generate_pdf()
                .expect("WinAnsi covers Spanish labels"),
        );
        assert!(spanish.contains("Recibido de: ") && spanish.contains("Descripción | Importe"));
        // A label outside WinAnsi needs a font, like any other text.
        let mut labels = crate::InvoiceLocale::En.labels();
        labels.title = "Квитанция".to_string();
        assert!(matches!(
            invoice("Plan").with_labels(labels).generate_pdf(),
            Err(CapitalError::InvalidInvoice(_))
        ));
    }

    #[test]
    fn native_pdf_is_parseable_and_preserves_winansi_invoice_text() {
        let bytes = invoice("Serviço de hospedagem")
            .generate_pdf()
            .expect("bounded PDF");
        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse generated PDF");
        assert_eq!(parsed.page_count(), 1);
        let text = parsed
            .extract_text()
            .into_iter()
            .flatten()
            .collect::<String>();
        assert!(text.contains("INV-2026-1"));
        assert!(text.contains("Serviço de hospedagem"));
        assert!(text.contains("49.90 BRL"));
    }

    #[test]
    fn default_font_rejects_unsupported_scripts_and_custom_font_is_checked() {
        assert!(matches!(
            invoice("Curso 日本語").generate_pdf(),
            Err(CapitalError::InvalidInvoice(_))
        ));
        assert!(matches!(
            invoice("Subscription").generate_pdf_with_font(b"not a font"),
            Err(CapitalError::InvalidInvoice(_))
        ));

        let font = BuiltinFont::Helvetica.get_subset_font();
        let bytes = invoice("Serviço")
            .generate_pdf_with_font(&font.bytes)
            .expect("caller-supplied font");
        assert!(bytes.starts_with(b"%PDF-"));
        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse custom-font PDF");
        assert!(
            parsed
                .extract_text()
                .into_iter()
                .flatten()
                .collect::<String>()
                .contains("Serviço")
        );
        assert!(matches!(
            invoice("日本語").generate_pdf_with_font(&font.bytes),
            Err(CapitalError::InvalidInvoice(_))
        ));
    }

    fn invalid_message(result: Result<Vec<u8>, CapitalError>) -> String {
        match result {
            Err(CapitalError::InvalidInvoice(message)) => message,
            other => panic!("expected an invalid invoice, got {other:?}"),
        }
    }

    #[test]
    fn custom_font_size_is_checked_before_parsing() {
        let invoice = invoice("Subscription");
        let size = "PDF font must contain 1 byte to 8 MiB";
        let unparsable = "PDF font could not be parsed";
        assert_eq!(invalid_message(invoice.generate_pdf_with_font(b"")), size);
        assert_eq!(
            invalid_message(invoice.generate_pdf_with_font(&vec![0; 8_388_608])),
            unparsable
        );
        assert_eq!(
            invalid_message(invoice.generate_pdf_with_font(&vec![0; 8_388_609])),
            size
        );
    }

    #[test]
    fn a_custom_font_document_may_exceed_one_mebibyte() {
        // Font parsers ignore bytes after the last table, but the whole font
        // program is embedded, and pseudo-random bytes do not compress.
        let mut font = BuiltinFont::Helvetica.get_subset_font().bytes;
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        font.extend((0..1_200_000).map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        }));
        let bytes = invoice("Subscription")
            .generate_pdf_with_font(&font)
            .expect("a document below 16 MiB");
        assert!(bytes.len() > 1_100_000, "{}", bytes.len());
    }

    #[test]
    fn wrapped_description_keeps_its_amount_on_the_first_line() {
        let description = format!("{}{}", "a".repeat(64), "tail");
        let bytes = invoice(&description).generate_pdf().expect("bounded PDF");
        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse generated PDF");
        let text = parsed
            .extract_text()
            .into_iter()
            .flatten()
            .collect::<String>();
        assert!(
            text.contains(&format!("{} | 49.90 BRL\r\n  tail", "a".repeat(64))),
            "{text:?}"
        );
        assert!(
            text.starts_with("Invoice INV-2026-1\r\nBilled To: "),
            "{text:?}"
        );
        // One line break between consecutive lines and none before the first.
        let operations = &parsed.pages[0].ops;
        let breaks = operations
            .iter()
            .filter(|operation| matches!(operation, Op::AddLineBreak))
            .count();
        let shown = operations
            .iter()
            .filter(|operation| matches!(operation, Op::ShowText { .. }))
            .count();
        assert_eq!(shown, 9);
        assert_eq!(breaks, 8);
    }

    #[test]
    fn renderer_paginates_bounded_long_invoices() {
        let mut invoice = invoice("Subscription");
        invoice.items = (0..100)
            .map(|index| InvoiceItem::new(format!("Line item {index}"), 100))
            .collect();
        let bytes = invoice.generate_pdf().expect("multipage PDF");
        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse multipage PDF");
        assert!(parsed.page_count() >= 3);
    }
}
