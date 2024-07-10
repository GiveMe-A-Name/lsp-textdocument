use crate::FullTextDocument;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Range, Uri,
};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct TextDocuments(BTreeMap<Uri, FullTextDocument>);

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
        Self(BTreeMap::new())
    }

    pub fn documents(&self) -> &BTreeMap<Uri, FullTextDocument> {
        &self.0
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
        self.0.get(uri)
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
        self.0.get(uri).map(|document| document.get_content(range))
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
        self.0.get(uri).map(|document| document.language_id())
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

                let document = FullTextDocument::new(
                    text_document.language_id,
                    text_document.version,
                    text_document.text,
                );
                self.0.insert(text_document.uri, document);
                true
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(params.clone())
                    .expect("Expect receive DidChangeTextDocumentParams");

                if let Some(document) = self.0.get_mut(&params.text_document.uri) {
                    let changes = &params.content_changes;
                    let version = params.text_document.version;
                    document.update(changes, version);
                };
                true
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(params.clone())
                    .expect("Expect receive DidCloseTextDocumentParams");

                self.0.remove(&params.text_document.uri);
                true
            }
            _ => {
                // ignore other request
                false
            }
        }
    }
}
