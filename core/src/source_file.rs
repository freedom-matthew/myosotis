use std::io::{Cursor, Read};
use std::panic::{self, AssertUnwindSafe};

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

const DOCX_BODY: &str = "word/document.xml";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    #[error("Only .txt, .md, .pdf and .docx files can be opened.")]
    UnsupportedFileType,
    #[error("This file could not be read.")]
    Unreadable,
    #[error("No text found in this file. It may be a scanned document.")]
    NoText,
}

/// Turns the bytes of an opened file into plain text.
pub fn read_source(name: &str, bytes: &[u8]) -> Result<String, SourceError> {
    let extension: String = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .unwrap_or_default();
    let text: String = match extension.as_str() {
        "txt" | "md" => String::from_utf8_lossy(bytes).into_owned(),
        "pdf" => read_pdf(bytes)?,
        "docx" => read_docx(bytes)?,
        _ => return Err(SourceError::UnsupportedFileType),
    };
    if text.trim().is_empty() {
        return Err(SourceError::NoText);
    }
    Ok(text.trim().to_string())
}

fn read_pdf(bytes: &[u8]) -> Result<String, SourceError> {
    // The PDF library can panic on malformed files; that mustn't take the app down.
    panic::catch_unwind(AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem(bytes)
    }))
    .map_err(|_| SourceError::Unreadable)?
    .map_err(|_| SourceError::Unreadable)
}

fn read_docx(bytes: &[u8]) -> Result<String, SourceError> {
    let mut archive: ZipArchive<Cursor<&[u8]>> =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| SourceError::Unreadable)?;
    let mut xml: String = String::new();
    archive
        .by_name(DOCX_BODY)
        .map_err(|_| SourceError::Unreadable)?
        .read_to_string(&mut xml)
        .map_err(|_| SourceError::Unreadable)?;
    docx_text(&xml)
}

fn docx_text(xml: &str) -> Result<String, SourceError> {
    let mut reader: Reader<&[u8]> = Reader::from_str(xml);
    let mut text: String = String::new();
    let mut in_text_run: bool = false;
    loop {
        match reader.read_event().map_err(|_| SourceError::Unreadable)? {
            Event::Start(tag) if tag.name().as_ref() == "w:t" => in_text_run = true,
            Event::End(tag) if tag.name().as_ref() == "w:t" => in_text_run = false,
            Event::End(tag) if tag.name().as_ref() == "w:p" => text.push('\n'),
            Event::Text(run) if in_text_run => text.push_str(&run),
            Event::GeneralRef(entity) if in_text_run => {
                text.push_str(&decode_entity(&entity));
            }
            Event::Eof => return Ok(text),
            _ => {}
        }
    }
}

fn decode_entity(name: &str) -> String {
    match name {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docx_paragraphs_become_lines_and_entities_are_decoded() {
        let xml: &str = r#"<w:document><w:p><w:r><w:t>Cats &amp; dogs</w:t></w:r></w:p>
            <w:p><w:r><w:t>Second</w:t></w:r></w:p></w:document>"#;
        assert_eq!(docx_text(xml).unwrap(), "Cats & dogs\nSecond\n");
    }
}
