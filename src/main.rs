use clap::Parser;
use tower_lsp::LspService;
use tower_lsp::Server;

use crate::lsp::{ClientSettings, Lsp, ServerSettings};

mod lsp;
mod plugins;

#[derive(Parser)]
struct Cli {}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Lsp {
        client,
        client_settings: ClientSettings::new(),
        server_settings: ServerSettings::new(),
    })
    .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
