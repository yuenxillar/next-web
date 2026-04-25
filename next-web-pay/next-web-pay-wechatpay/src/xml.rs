use std::collections::BTreeMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::Result;

/// Builds a flat WeChat Pay XML payload.
pub fn build_xml(params: &BTreeMap<String, String>) -> String {
    let mut xml = String::from("<xml>");
    for (key, value) in params {
        xml.push('<');
        xml.push_str(key);
        xml.push_str("><![CDATA[");
        xml.push_str(&escape_cdata(value));
        xml.push_str("]]></");
        xml.push_str(key);
        xml.push('>');
    }
    xml.push_str("</xml>");
    xml
}

/// Parses a flat WeChat Pay XML payload into a map.
pub fn parse_xml(xml: &str) -> Result<BTreeMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_key: Option<String> = None;
    let mut values = BTreeMap::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(event) => {
                let key = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if key != "xml" {
                    current_key = Some(key);
                }
            }
            Event::Text(event) => {
                if let Some(key) = current_key.as_ref() {
                    values.insert(key.clone(), event.unescape()?.into_owned());
                }
            }
            Event::CData(event) => {
                if let Some(key) = current_key.as_ref() {
                    values.insert(
                        key.clone(),
                        String::from_utf8_lossy(event.into_inner().as_ref()).to_string(),
                    );
                }
            }
            Event::End(event) => {
                if event.name().as_ref() != b"xml" {
                    current_key = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(values)
}

fn escape_cdata(value: &str) -> String {
    value.replace("]]>", "]]]]><![CDATA[>")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{build_xml, parse_xml};

    #[test]
    fn xml_roundtrip_works() {
        let mut params = BTreeMap::new();
        params.insert("appid".to_string(), "wx123".to_string());
        params.insert("body".to_string(), "book".to_string());

        let xml = build_xml(&params);
        let parsed = parse_xml(&xml).expect("parse");

        assert_eq!(parsed.get("appid").map(String::as_str), Some("wx123"));
        assert_eq!(parsed.get("body").map(String::as_str), Some("book"));
    }
}
