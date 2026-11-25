//! The fonts a document uses, read from the file itself (no app needed, on every system):
//! - Word, Excel, PowerPoint (.docx .xlsx .pptx and their templates): `w:font w:name`, `<name val>`,
//!   `typeface="…"` in the package's XML parts;
//! - OpenDocument (.odt .ods .odp .odg and the flat .fodt .fods .fodp .fodg): `style:font-face`
//!   `svg:font-family`, `fo:font-family`, `style:font-name`;
//! - RTF: the font table; SVG and HTML: `font-family`;
//! - PressCraft (its PDFs and .presscraft files): the families of its document.json.
//! Generic families (serif, sans-serif…), theme placeholders (+mn-lt…) and the system's UI font are
//! left out.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

const GENERIC: &[&str] = &["serif", "sans-serif", "monospace", "cursive", "fantasy", "system-ui", "inherit", "initial", "default"];

/// The document kinds this reads.
pub fn readable(path: &Path) -> bool {
    matches!(
        ext(path).as_str(),
        "docx" | "dotx" | "docm" | "xlsx" | "xltx" | "xlsm" | "pptx" | "potx" | "pptm" | "odt" | "ott" | "ods" | "ots" | "odp" | "otp" | "odg" | "otg" | "fodt" | "fods" | "fodp" | "fodg" | "rtf" | "svg" | "html" | "htm"
    )
}

fn ext(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// Every font family the document names, sorted.
pub fn fonts_in(path: &Path) -> Vec<String> {
    let mut out = BTreeSet::new();
    let e = ext(path);
    match e.as_str() {
        "docx" | "dotx" | "docm" | "xlsx" | "xltx" | "xlsm" | "pptx" | "potx" | "pptm" | "odt" | "ott" | "ods" | "ots" | "odp" | "otp" | "odg" | "otg" => {
            let Ok(f) = std::fs::File::open(path) else { return Vec::new() };
            let Ok(mut z) = zip::ZipArchive::new(f) else { return Vec::new() };
            for i in 0..z.len() {
                let Ok(mut entry) = z.by_index(i) else { continue };
                let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
                if !name.ends_with(".xml") || entry.size() > 64 << 20 {
                    continue;
                }
                let mut text = String::new();
                if entry.read_to_string(&mut text).is_ok() {
                    xml_fonts(&text, &mut out);
                }
            }
        }
        "fodt" | "fods" | "fodp" | "fodg" | "svg" | "html" | "htm" => {
            if let Ok(text) = std::fs::read_to_string(path) {
                xml_fonts(&text, &mut out);
                css_fonts(&text, &mut out);
            }
        }
        "rtf" => {
            if let Ok(bytes) = std::fs::read(path) {
                rtf_fonts(&String::from_utf8_lossy(&bytes), &mut out);
            }
        }
        // PressCraft's documents: a PDF carrying the editable document, or that document alone
        // (presscraft.rs); any other PDF names nothing here.
        "pdf" | "presscraft" => {
            if std::fs::metadata(path).is_ok_and(|m| m.len() <= 1 << 30) {
                if let Ok(bytes) = std::fs::read(path) {
                    for f in crate::presscraft::families(&bytes) {
                        keep(&f, &mut out);
                    }
                }
            }
        }
        _ => {}
    }
    out.into_iter().collect()
}

/// The document kinds read for this app: PressCraft's PDFs and .presscraft files besides the others.
pub fn readable_for(path: &Path, bundle: &str) -> bool {
    readable(path) || (bundle == crate::presscraft::BUNDLE && matches!(ext(path).as_str(), "pdf" | "presscraft"))
}

fn keep(name: &str, out: &mut BTreeSet<String>) {
    let decoded = name.replace("&apos;", "'").replace("&quot;", "\"").replace("&amp;", "&");
    let n = decoded.trim().trim_matches(|c| c == '\'' || c == '"').trim();
    if n.is_empty() || n.starts_with('+') || n.starts_with('.') || n.len() > 80 || GENERIC.contains(&n.to_ascii_lowercase().as_str()) {
        return;
    }
    out.insert(n.to_string());
}

/// Attribute values `attr="…"` in XML.
fn attr_values<'a>(text: &'a str, attr: &str) -> Vec<&'a str> {
    let mut v = Vec::new();
    let pat = format!("{attr}=\"");
    let mut rest = text;
    while let Some(i) = rest.find(&pat) {
        // The attribute name must start a word (not "xyzw:name").
        let ok = i == 0 || rest[..i].ends_with(|c: char| c.is_whitespace());
        rest = &rest[i + pat.len()..];
        if let Some(end) = rest.find('"') {
            if ok {
                v.push(&rest[..end]);
            }
            rest = &rest[end..];
        }
    }
    v
}

fn xml_fonts(text: &str, out: &mut BTreeSet<String>) {
    // Word: <w:rFonts w:ascii="…"> and <w:font w:name="…">.
    for a in ["w:ascii", "w:hAnsi", "w:cs", "w:eastAsia"] {
        for v in attr_values(text, a) {
            keep(v, out);
        }
    }
    if text.contains("<w:font ") {
        for chunk in text.split("<w:font ").skip(1) {
            if let Some(v) = attr_values(chunk.split('>').next().unwrap_or(""), "w:name").first() {
                keep(v, out);
            }
        }
    }
    // PowerPoint / themes: typeface="…".
    for v in attr_values(text, "typeface") {
        keep(v, out);
    }
    // Excel: <font><name val="…"/></font>.
    for chunk in text.split("<name val=\"").skip(1) {
        if let Some(v) = chunk.split('"').next() {
            keep(v, out);
        }
    }
    // OpenDocument.
    for a in ["svg:font-family", "fo:font-family", "style:font-family-generic-fallback"] {
        if a.ends_with("fallback") {
            continue;
        }
        for v in attr_values(text, a) {
            for part in v.split(',') {
                keep(part, out);
            }
        }
    }
    // SVG text attributes.
    for v in attr_values(text, "font-family") {
        for part in v.split(',') {
            keep(part, out);
        }
    }
}

fn css_fonts(text: &str, out: &mut BTreeSet<String>) {
    for chunk in text.split("font-family:").skip(1) {
        let v = chunk.split([';', '"', '}', '\n']).next().unwrap_or("");
        for part in v.split(',') {
            keep(part, out);
        }
    }
}

fn rtf_fonts(text: &str, out: &mut BTreeSet<String>) {
    let Some(start) = text.find("{\\fonttbl") else { return };
    let mut depth = 0i32;
    let mut end = start;
    for (i, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    for entry in text[start..end].split("{\\f").skip(1) {
        // "{\f0\fswiss\fcharset0 Helvetica;}": the name is what follows the last control word.
        let body = entry.split(';').next().unwrap_or("");
        let name = body.rsplit(['\\', '}']).next().unwrap_or("");
        let name = name.split_once(' ').map(|(_, n)| n).unwrap_or(name);
        keep(name, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtf_table() {
        let mut out = BTreeSet::new();
        rtf_fonts("{\\rtf1{\\fonttbl{\\f0\\fswiss\\fcharset0 Helvetica;}{\\f1\\froman Times New Roman;}}hi}", &mut out);
        assert!(out.contains("Helvetica") && out.contains("Times New Roman"));
    }

    #[test]
    fn xml_attrs() {
        let mut out = BTreeSet::new();
        xml_fonts("<w:rFonts w:ascii=\"Lobster\" w:hAnsi=\"Lobster\"/><a:latin typeface=\"+mn-lt\"/><style:font-face style:name=\"X\" svg:font-family=\"&apos;Open Sans&apos;\"/>", &mut out);
        assert!(out.contains("Lobster"));
        assert!(!out.iter().any(|f| f.starts_with('+')));
        assert!(out.iter().any(|f| f.contains("Open Sans")));
    }
}
