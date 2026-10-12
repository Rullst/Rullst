use super::MultipartError;
use roxmltree::{Document, Node, ParsingOptions};

pub(crate) fn parse<'a>(bytes: &'a [u8], root: &str) -> Result<Document<'a>, MultipartError> {
    if bytes.len() > 256 * 1024 {
        return Err(MultipartError::InvalidResponse);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| MultipartError::InvalidResponse)?;
    let doc = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 4096,
            ..Default::default()
        },
    )
    .map_err(|_| MultipartError::InvalidResponse)?;
    if doc.root_element().tag_name().name() != root {
        return Err(MultipartError::InvalidResponse);
    }
    for node in doc.descendants().filter(Node::is_element) {
        if !matches!(
            node.tag_name().namespace(),
            None | Some("http://s3.amazonaws.com/doc/2006-03-01/")
        ) || node.ancestors().take(9).count() > 8
        {
            return Err(MultipartError::InvalidResponse);
        }
    }
    Ok(doc)
}

pub(crate) fn field<'a>(node: Node<'a, 'a>, name: &str) -> Result<&'a str, MultipartError> {
    let mut values = node
        .children()
        .filter(|v| v.is_element() && v.tag_name().name() == name);
    let value = values.next().ok_or(MultipartError::InvalidResponse)?;
    if values.next().is_some() || value.children().any(|v| v.is_element()) {
        return Err(MultipartError::InvalidResponse);
    }
    value.text().ok_or(MultipartError::InvalidResponse)
}

pub(super) fn valid_etag(value: &str) -> bool {
    value.len() >= 2
        && value.len() <= 256
        && value.starts_with('"')
        && value.ends_with('"')
        && value[1..value.len() - 1]
            .bytes()
            .all(|c| c.is_ascii_graphic() && !matches!(c, b'"' | b'<' | b'>' | b'&' | b'\''))
}

pub(crate) fn escaped(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(super) struct ListedPart {
    pub number: u16,
    pub size: u64,
    pub etag: String,
}

pub(super) fn parts(
    bytes: &[u8],
    bucket: &str,
    key: &str,
    upload: &str,
) -> Result<Vec<ListedPart>, MultipartError> {
    let doc = parse(bytes, "ListPartsResult")?;
    let root = doc.root_element();
    if field(root, "Bucket")? != bucket
        || field(root, "Key")? != key
        || field(root, "UploadId")? != upload
        || field(root, "IsTruncated")? != "false"
    {
        return Err(MultipartError::InvalidResponse);
    }
    let mut parts = Vec::new();
    let mut previous = 0;
    for node in root
        .children()
        .filter(|v| v.is_element() && v.tag_name().name() == "Part")
    {
        let number = field(node, "PartNumber")?
            .parse::<u16>()
            .map_err(|_| MultipartError::InvalidResponse)?;
        let size = field(node, "Size")?
            .parse::<u64>()
            .map_err(|_| MultipartError::InvalidResponse)?;
        let etag = field(node, "ETag")?;
        if number <= previous || number > 256 || !valid_etag(etag) {
            return Err(MultipartError::InvalidResponse);
        }
        previous = number;
        parts.push(ListedPart {
            number,
            size,
            etag: etag.into(),
        });
    }
    Ok(parts)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const NS: &str = "http://s3.amazonaws.com/doc/2006-03-01/";

    fn listing(entries: &str) -> String {
        format!(
            "<ListPartsResult xmlns=\"{NS}\"><Bucket>b</Bucket><Key>k</Key><UploadId>u</UploadId><IsTruncated>false</IsTruncated>{entries}</ListPartsResult>"
        )
    }

    fn nested(depth: usize) -> String {
        let mut xml = String::from("<R>");
        for _ in 1..depth {
            xml.push_str("<e>");
        }
        for _ in 1..depth {
            xml.push_str("</e>");
        }
        xml.push_str("</R>");
        xml
    }

    #[test]
    fn documents_are_bounded_in_bytes_nodes_and_depth() {
        let padded = |len: usize| format!("<R>{}</R>", " ".repeat(len - 7));
        assert!(parse(padded(262_144).as_bytes(), "R").is_ok());
        assert!(parse(padded(262_145).as_bytes(), "R").is_err());
        let crowded = format!("<R>{}</R>", "<e/>".repeat(5_000));
        assert!(parse(crowded.as_bytes(), "R").is_err());
        assert!(parse(nested(7).as_bytes(), "R").is_ok());
        assert!(parse(nested(8).as_bytes(), "R").is_err());
    }

    #[test]
    fn etags_are_short_quoted_and_plain() {
        for valid in ["\"\"", "\"abc-1\"", &format!("\"{}\"", "a".repeat(254))] {
            assert!(valid_etag(valid), "{valid}");
        }
        for invalid in [
            "abc",
            "x\"",
            "\"x",
            &format!("\"{}\"", "a".repeat(255)),
            "\"a<b\"",
            "\"a b\"",
        ] {
            assert!(!valid_etag(invalid), "{invalid}");
        }
    }

    #[test]
    fn listed_parts_must_belong_to_this_upload_and_be_in_range() {
        let part = |number: u32, etag: &str| {
            format!(
                "<Part><PartNumber>{number}</PartNumber><Size>5</Size><ETag>{etag}</ETag></Part>"
            )
        };
        let listed = parts(listing(&part(256, "\"e\"")).as_bytes(), "b", "k", "u").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].number, 256);
        for entries in [part(257, "\"e\""), part(1, "e")] {
            assert!(parts(listing(&entries).as_bytes(), "b", "k", "u").is_err());
        }
        let xml = listing("");
        assert!(parts(xml.as_bytes(), "b", "other", "u").is_err());
        assert!(parts(xml.as_bytes(), "b", "k", "other").is_err());
    }
}
