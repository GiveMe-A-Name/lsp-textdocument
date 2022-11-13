use crate::FullTextDocument;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification,
    },
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Range, Url,
};
use serde_json::Value;
use std::collections::HashMap;

pub struct TextDocuments(HashMap<Url, FullTextDocument>);

impl TextDocuments {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn documents(&self) -> &HashMap<Url, FullTextDocument> {
        &self.0
    }

    pub fn get_document(&self, uri: &Url) -> Option<&FullTextDocument> {
        self.0.get(uri)
    }

    pub fn get_document_content(&self, uri: &Url, range: Option<Range>) -> Option<&str> {
        self.0.get(uri).map(|document| document.get_content(range))
    }

    pub fn get_document_language(&self, uri: &Url) -> Option<&str> {
        self.0.get(uri).map(|document| document.language_id())
    }

    pub fn listen(&mut self, method: &str, params: Value) {
        match method {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(params)
                    .expect("Expect receive DidOpenTextDocumentParams");
                let text_document = params.text_document;

                let document = FullTextDocument::new(
                    text_document.language_id,
                    text_document.version,
                    text_document.text,
                );
                self.0.insert(text_document.uri, document);
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = serde_json::from_value(params)
                    .expect("Expect receive DidChangeTextDocumentParams");

                if let Some(document) = self.0.get_mut(&params.text_document.uri) {
                    let changes = &params.content_changes;
                    let version = params.text_document.version;
                    document.update(changes, version);
                };
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(params)
                    .expect("Expect receive DidCloseTextDocumentParams");

                self.0.remove(&params.text_document.uri);
            }
            _ => {
                // ignore other request
            }
        }
    }
}
