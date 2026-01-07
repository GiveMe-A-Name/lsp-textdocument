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
use lsp_types::Uri;

fn main() {
    // Defaults to UTF-16 (per LSP backward compatibility)
    let mut text_documents = TextDocuments::new();

    // Or explicitly choose the negotiated encoding
    // use lsp_types::PositionEncodingKind;
    // let mut text_documents = TextDocuments::with_encoding(PositionEncodingKind::UTF8);

    let uri: Uri = "file:///example.txt".parse().unwrap();
    let text = text_documents.get_document_content(&uri, None);
    println!("{:?}", text);
}
```

### with [`lsp-server`](https://github.com/rust-analyzer/lsp-server)

[`examples/with_lsp_server.rs`](/examples/with_lsp_server.rs)

### with [`tower-lsp`](https://github.com/ebkalderon/tower-lsp)

**Contact us via [issues](https://github.com/GiveMe-A-Name/lsp-textdocument/issues) if you require this with `tower-lsp`**

## Position encodings

- Supports all LSP 3.17 position encodings: UTF-8, UTF-16, and UTF-32.
- Default is UTF-16 to stay backward compatible; use `TextDocuments::with_encoding` or `FullTextDocument::new_with_encoding` to opt into UTF-8/UTF-32.
