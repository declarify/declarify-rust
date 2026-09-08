//! Handwritten server entry point. Wires the generated `axum` routes to a
//! SQLite-backed repository via `composition::open_sqlite_note_repository`.
//! Nothing about routing, persistence, or the service is defined here —
//! this binary only chooses the backend and binds a socket, per
//! `docs/06-extension-model.md` ("backend selection is handwritten
//! composition").

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;

/// Declarify Step 0 `notes` REST server.
#[derive(Parser)]
#[command(name = "notes-server", version, about)]
struct Args {
    /// Path to the SQLite database file (created if it does not exist).
    #[arg(long, env = "NOTES_DB_PATH", default_value = "notes.sqlite3")]
    db: PathBuf,

    /// Address to listen on.
    #[arg(long, env = "NOTES_LISTEN_ADDR", default_value = "127.0.0.1:8080")]
    listen: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let repository = notes::composition::open_sqlite_note_repository(&args.db)
        .unwrap_or_else(|err| panic!("failed to open SQLite database {:?}: {err}", args.db));
    let service: Arc<dyn notes::generated::application::note::NoteService> =
        Arc::new(notes::composition::build_note_service(repository));
    let router = notes::generated::http::note::routes(service);

    let listener = tokio::net::TcpListener::bind(&args.listen)
        .await
        .unwrap_or_else(|err| panic!("failed to bind {}: {err}", args.listen));
    eprintln!("notes-server listening on http://{}", args.listen);

    axum::serve(listener, router)
        .await
        .expect("server terminated unexpectedly");
}
