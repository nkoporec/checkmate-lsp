



use tower_lsp::LspService;
use tower_lsp::Server;
use flexi_logger::{FileSpec, Logger, WriteMode};
use clap::Parser;
use simple_home_dir::home_dir;

use crate::{lsp::{Lsp, ClientSettings, ServerSettings }};

mod lsp;
mod plugins;

#[derive(Parser)]
struct Cli {
    #[clap(long, short, action)]
    debug: bool,
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let args = Cli::parse();

    if args.debug {
        let home_dir = home_dir().expect("Can't fetch home dir.");
        let logger_file = FileSpec::try_from(format!("{}/checkmate.log", home_dir.to_str().unwrap())).unwrap();
        let _logger = Logger::try_with_str("info")
            .unwrap()
            .log_to_file(logger_file)
            .write_mode(WriteMode::BufferAndFlush)
            .start()
            .unwrap();
    }

    let (service, socket) = LspService::build(|client| Lsp {
        client,
        client_settings: ClientSettings::new(),
        server_settings: ServerSettings::new(args.debug),
    })
    .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
