use myosotis_core::{SourceError, read_source};

#[test]
fn a_text_file_is_read_as_it_is() {
    // given a plain text file
    let bytes: &[u8] = b"Rust has ownership.";

    // when I open it
    let text: Result<String, SourceError> = read_source("notes.txt", bytes);

    // then I get its text
    assert_eq!(text, Ok("Rust has ownership.".to_string()));
}

#[test]
fn a_markdown_file_is_read_as_text() {
    // given a Markdown file
    let bytes: &[u8] = b"# Ownership\n\nRust has ownership.";

    // when I open it
    let text: String = read_source("notes.MD", bytes).unwrap();

    // then I get its text, whatever the case of the extension
    assert!(text.contains("Rust has ownership."));
}

#[test]
fn a_pdf_gives_its_text() {
    // given a real PDF
    let bytes: &[u8] = include_bytes!("fixtures/source.pdf");

    // when I open it
    let text: String = read_source("source.pdf", bytes).unwrap();

    // then its sentences are in the text
    assert!(text.contains("Rust has ownership."));
    assert!(text.contains("never outlives"));
}

#[test]
fn a_word_document_gives_its_text() {
    // given a real .docx file
    let bytes: &[u8] = include_bytes!("fixtures/source.docx");

    // when I open it
    let text: String = read_source("source.docx", bytes).unwrap();

    // then its sentences are in the text
    assert!(text.contains("Rust has ownership."));
    assert!(text.contains("never outlives"));
}

#[test]
fn other_file_types_are_rejected() {
    // given a file type the app can't read
    let result: Result<String, SourceError> = read_source("slides.pptx", b"whatever");

    // then it is rejected
    assert_eq!(result, Err(SourceError::UnsupportedFileType));
}

#[test]
fn a_damaged_pdf_is_reported_rather_than_crashing() {
    // given bytes that only claim to be a PDF
    let result: Result<String, SourceError> = read_source("broken.pdf", b"%PDF-not really");

    // then I get an error
    assert_eq!(result, Err(SourceError::Unreadable));
}

#[test]
fn an_empty_file_is_reported() {
    // given a file with nothing in it
    let result: Result<String, SourceError> = read_source("empty.txt", b"   \n");

    // then I am told there is no text
    assert_eq!(result, Err(SourceError::NoText));
}
