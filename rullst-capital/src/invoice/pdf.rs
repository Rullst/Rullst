use super::{Invoice, ValidatedInvoiceMoney, invalid_invoice};
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
        let money = self.validated_money()?;
        let lines = self.render_lines(&money);
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
        let money = self.validated_money()?;
        let lines = self.render_lines(&money);
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

    fn render_lines(&self, money: &ValidatedInvoiceMoney) -> Vec<String> {
        let mut lines = vec![
            format!("Invoice {}", self.invoice_id),
            format!("Billed To: {}", self.customer_email),
            format!("Date: {}", self.date.format("%Y-%m-%d")),
            String::new(),
        ];
        let currency = self.currency.to_ascii_uppercase();
        for (item, amount_minor) in self.items.iter().zip(&money.item_amounts_minor) {
            let chunks = wrap_chars(&item.description, DESCRIPTION_CHARS_PER_LINE);
            for (index, chunk) in chunks.into_iter().enumerate() {
                if index == 0 {
                    lines.push(format!(
                        "{} | {}",
                        chunk,
                        format_money(*amount_minor, &currency)
                    ));
                } else {
                    lines.push(format!("  {chunk}"));
                }
            }
        }
        lines.push(String::new());
        lines.push(format!(
            "Total: {}",
            format_money(money.total_minor, &currency)
        ));
        lines
    }
}

fn render_pdf(
    invoice: &Invoice,
    lines: Vec<String>,
    external_font: Option<ParsedFont>,
) -> Result<Vec<u8>, CapitalError> {
    let mut document = PdfDocument::new(&format!("Invoice {}", invoice.invoice_id));
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

fn format_money(amount_minor: u64, currency: &str) -> String {
    format!(
        "{}.{:02} {currency}",
        amount_minor / 100,
        amount_minor % 100
    )
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
        Invoice {
            invoice_id: "INV-2026-1".to_string(),
            customer_email: "cliente@example.com".to_string(),
            date: Utc::now(),
            items: vec![InvoiceItem {
                description: description.to_string(),
                amount: 49.90,
            }],
            total: 49.90,
            currency: "brl".to_string(),
        }
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

    #[test]
    fn renderer_paginates_bounded_long_invoices() {
        let mut invoice = invoice("Subscription");
        invoice.items = (0..100)
            .map(|index| InvoiceItem {
                description: format!("Line item {index}"),
                amount: 1.0,
            })
            .collect();
        invoice.total = 100.0;
        let bytes = invoice.generate_pdf().expect("multipage PDF");
        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse multipage PDF");
        assert!(parsed.page_count() >= 3);
    }
}
