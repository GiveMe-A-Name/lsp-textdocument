//!
//! A LSP text documents manager that helps mapping of text document.
//!
//! The text documents [position-encoding](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#positionEncodingKind) only supports `UTF-16`

mod text_document;
mod text_documents;

pub use text_document::FullTextDocument;
pub use text_documents::TextDocuments;
