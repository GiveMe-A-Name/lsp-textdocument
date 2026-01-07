use crate::FullTextDocument;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, PositionEncodingKind,
    Range, Uri,
};
use serde_json::Value;
use std::collections::BTreeMap;

pub struct TextDocuments {
    documents: BTreeMap<Uri, FullTextDocument>,
    default_encoding: PositionEncodingKind,
}

impl Default for TextDocuments {
    fn default() -> Self {
        Self::with_encoding(PositionEncodingKind::UTF16)
    }
}

impl TextDocuments {
    /// Create a text documents
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use lsp_textdocument::TextDocuments;
    ///
    /// let text_documents = TextDocuments::new();
    /// ```
    pub fn new() -> Self {
        Self::with_encoding(PositionEncodingKind::UTF16)
    }

    /// Create a TextDocuments instance with a specific position encoding
    ///
    /// This method allows you to specify the position encoding used for character positions
    /// in text documents. The encoding determines how character offsets are calculated and is
    /// important for proper LSP communication between client and server.
    ///
    /// # Arguments
    ///
    /// * `default_encoding` - The position encoding to use. Can be UTF-8, UTF-16, or UTF-32.
    ///
    /// # Position Encodings
    ///
    /// - **UTF-16**: The default encoding for backward compatibility with LSP 3.16 and earlier.
    ///   Each UTF-16 code unit counts as one position unit.
    /// - **UTF-8**: Each byte counts as one position unit. More efficient for ASCII-heavy text.
    /// - **UTF-32**: Each Unicode code point counts as one position unit.
    ///
    /// The encoding should match what was negotiated with the LSP client during initialization.
    ///
    /// # Examples
    ///
    /// Basic usage with UTF-16 (default):
    ///
    /// ```
    /// use lsp_textdocument::TextDocuments;
    /// use lsp_types::PositionEncodingKind;
    ///
    /// let text_documents = TextDocuments::with_encoding(PositionEncodingKind::UTF16);
    /// ```
    ///
    /// Using UTF-8 encoding for better performance with ASCII text:
    ///
    /// ```
    /// use lsp_textdocument::TextDocuments;
    /// use lsp_types::PositionEncodingKind;
    ///
    /// let text_documents = TextDocuments::with_encoding(PositionEncodingKind::UTF8);
    /// ```
    ///
    /// Using UTF-32 encoding where each Unicode code point is one unit:
    ///
    /// ```
    /// use lsp_textdocument::TextDocuments;
    /// use lsp_types::PositionEncodingKind;
    ///
    /// let text_documents = TextDocuments::with_encoding(PositionEncodingKind::UTF32);
    /// ```
    pub fn with_encoding(default_encoding: PositionEncodingKind) -> Self {
        Self {
            documents: BTreeMap::new(),
            default_encoding,
        }
    }

    pub fn documents(&self) -> &BTreeMap<Uri, FullTextDocument> {
        &self.documents
    }

    /// Returns the default position encoding used for newly created documents.
    ///
    /// This is useful for checking which encoding was configured or negotiated
    /// (for example, during server initialization) when this `TextDocuments`
    /// instance was created.
    pub fn default_encoding(&self) -> PositionEncodingKind {
        self.default_encoding.clone()
    }

    /// Get specify document by giving Uri
    ///
    /// # Examples:
    ///
    /// Basic usage:
    /// ```
    /// use lsp_textdocument::TextDocuments;
    /// use lsp_types::Uri;
    ///
    /// let text_documents = TextDocuments::new();
    /// let uri:Uri = "file://example.txt".parse().unwrap();
    /// text_documents.get_document(&uri);
    /// ```
    pub fn get_document(&self, uri: &Uri) -> Option<&FullTextDocument> {
        self.documents.get(uri)
    }

    /// Get specify document content by giving Range
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```no_run
    /// use lsp_textdocument::TextDocuments;
    /// use lsp_types::{Uri, Range, Position};
    ///
    /// let uri: Uri = "file://example.txt".parse().unwrap();
    /// let text_documents = TextDocuments::new();
    ///
    /// // get document all content
    /// let content = text_documents.get_document_content(&uri, None);
    /// assert_eq!(content, Some("hello rust!"));
    ///
    /// // get document specify content by range
    /// let (start, end) = (Position::new(0, 1), Position::new(0, 9));
    /// let range = Range::new(start, end);
    /// let sub_content = text_documents.get_document_content(&uri, Some(range));
    /// assert_eq!(sub_content, Some("ello rus"));
    /// ```
    pub fn get_document_content(&self, uri: &Uri, range: Option<Range>) -> Option<&str> {
        self.documents
            .get(uri)
            .map(|document| document.get_content(range))
    }

    /// Get specify document's language by giving Uri
    ///
    /// # Examples
    ///
    /// Basic usage:
    /// ```no_run
    /// use lsp_textdocument::TextDocuments;
    /// use lsp_types::Uri;
    ///
    /// let text_documents = TextDocuments::new();
    /// let uri:Uri = "file://example.js".parse().unwrap();
    /// let language =  text_documents.get_document_language(&uri);
    /// assert_eq!(language, Some("javascript"));
    /// ```
    pub fn get_document_language(&self, uri: &Uri) -> Option<&str> {
        self.documents
            .get(uri)
            .map(|document| document.language_id())
    }

    /// Listening the notification from client, you just need to pass `method` and `params`
    ///
    /// # Examples:
    ///
    /// Basic usage:
    /// ```no_run
    /// use lsp_textdocument::TextDocuments;
    ///
    /// let method = "textDocument/didOpen";
    /// let params = serde_json::to_value("message produced by client").unwrap();
    ///
    /// let mut text_documents = TextDocuments::new();
    /// let accept: bool = text_documents.listen(method, &params);
    /// ```
    pub fn listen(&mut self, method: &str, params: &Value) -> bool {
        match method {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(params.clone())
                    .expect("Expect receive DidOpenTextDocumentParams");
                let text_document = params.text_document;

                let document = FullTextDocument::new_with_encoding(
                    text_document.language_id,
                    text_document.version,
                    text_document.text,
                    // use default encoding negotiated at server init
                    self.default_encoding.clone(),
                );
                self.documents.insert(text_document.uri, document);
                true
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(params.clone())
                    .expect("Expect receive DidChangeTextDocumentParams");

                if let Some(document) = self.documents.get_mut(&params.text_document.uri) {
                    let changes = &params.content_changes;
                    let version = params.text_document.version;
                    document.update(changes, version);
                };
                true
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(params.clone())
                    .expect("Expect receive DidCloseTextDocumentParams");

                self.documents.remove(&params.text_document.uri);
                true
            }
            _ => {
                // ignore other request
                false
            }
        }
    }
}
