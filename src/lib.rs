//!
//! A LSP text documents manager that helps mapping of text document.
//!
//! Supports all LSP 3.17 position encodings (UTF-8/UTF-16/UTF-32); defaults to UTF-16 for backward compatibility.

mod text_document;
mod text_documents;

pub use text_document::FullTextDocument;
pub use text_documents::TextDocuments;
