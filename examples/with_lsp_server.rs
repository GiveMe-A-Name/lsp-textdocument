use anyhow::Result;
use lsp_server::{Connection, ExtractError, Message, Request, RequestId};
use lsp_textdocument::TextDocuments;
use lsp_types::request::Formatting;
use lsp_types::{HoverProviderCapability, OneOf, TextDocumentSyncCapability, TextDocumentSyncKind};
use lsp_types::{InitializeParams, ServerCapabilities};

fn main() -> Result<()> {
    // Note that  we must have our logging only write out to stderr.
    eprintln!("starting generic LSP server");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let mut documents = TextDocuments::new();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })?;
    let initialization_params = connection.initialize(server_capabilities)?;

    main_loop(connection, initialization_params, &mut documents)?;

    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}

fn main_loop(
    connection: Connection,
    params: serde_json::Value,
    documents: &mut TextDocuments,
) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    eprintln!("starting example main loop");
    for msg in connection.receiver.iter() {
        // eprintln!("got msg: {:?}", msg);
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                match cast::<Formatting>(req) {
                    std::result::Result::Ok((_id, params)) => {
                        let uri = params.text_document.uri;
                        let text = documents.get_document_content(&uri, None);

                        // !you can get document that handle by user content by using documents
                        eprintln!("the document text: {:?}", text);
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                    }
                }

                // ...
            }
            Message::Response(resp) => {
                eprintln!("got response: {:?}", resp);
            }
            Message::Notification(not) => {
                if !documents.listen(not.method.as_str(), &not.params) {
                    // Add handlers for other types of notifications here.
                }
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
