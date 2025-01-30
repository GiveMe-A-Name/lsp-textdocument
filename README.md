# `lsp-textdocument`

A LSP text documents manager that helps mapping of textual content.

## Introduction

You may not be able to manage your text documents comfortably when developing an LSP service. There are two reasons why we develop hard.

- Always given a URL variable only, so we need to read the contents of the file ourselves.
- Need map offsets from string index to text dimensional coordinates.

By listening to the notification from the LSP client, `lsp-textdocument` can help you automatically manage text documents.

This crate is base on [vscode-languageserver-textdocument](https://github.com/microsoft/vscode-languageserver-node/tree/main/textDocument).

## Example usage

### Basic usage

```rust
use lsp_textdocument::TextDocuments;

fn main() {
    let text_documents = TextDocument::new();
    ...


    let text = text_documents.get_document_content(&url, None);
}
```

### with [`lsp-server`](https://github.com/rust-analyzer/lsp-server)

[`examples/with_lsp_server.rs`](/examples/with_lsp_server.rs)

### with [`tower-lsp`](https://github.com/ebkalderon/tower-lsp)

**Contact us via [issues](https://github.com/GiveMe-A-Name/lsp-textdocument/issues) if you require this with `tower-lsp`**

## Attention

- The text documents [position-encoding](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#positionEncodingKind) only supports `UTF-16`

