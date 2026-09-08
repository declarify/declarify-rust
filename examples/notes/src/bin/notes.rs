//! Handwritten CLI entry point. Parses global flags (base URL) and hands off
//! to the generated `NoteCommand`/`run`, per `docs/05-rest-client-cli.md`
//! ("The CLI accepts the server base URL explicitly and from
//! configuration."). Everything about what commands exist and how they call
//! the REST API is generated; this binary only wires argument parsing,
//! process exit codes, and stderr reporting, none of which is
//! resource-specific business logic.

use clap::Parser;

use notes::generated::cli::note::NoteCommand;
use notes::generated::client::note::NoteClient;

/// Declarify Step 0 `notes` CLI.
#[derive(Parser)]
#[command(name = "notes", version, about)]
struct Cli {
    /// Base URL of a running notes-server.
    #[arg(long, env = "NOTES_BASE_URL", default_value = "http://127.0.0.1:8080")]
    base_url: String,

    #[command(subcommand)]
    command: NoteCommand,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = NoteClient::new(cli.base_url);

    if let Err(err) = notes::generated::cli::note::run(cli.command, &client).await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
