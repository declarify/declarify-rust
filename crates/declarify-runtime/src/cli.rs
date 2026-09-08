//! Shared, resource-agnostic CLI building blocks: JSON stdin/stdout I/O and
//! the error type generated commands return. See
//! `docs/05-rest-client-cli.md` ("CLI"): "Step 0 supports JSON input/output
//! as the stable automation interface." Errors here always map to a
//! non-zero process exit; the handwritten binary entry point decides the
//! exact code and prints to stderr.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::http::ClientError;

/// Everything that can go wrong running a generated CLI command.
#[derive(Debug)]
pub enum CliError {
    /// The REST call itself failed (transport, decode, or a structured API
    /// error from the server).
    Client(ClientError),
    /// Stdin was not valid JSON for the expected input shape.
    InvalidInput(serde_json::Error),
    /// Reading stdin or writing stdout failed.
    Io(std::io::Error),
}

impl From<ClientError> for CliError {
    fn from(err: ClientError) -> Self {
        CliError::Client(err)
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Client(err) => write!(f, "{err}"),
            CliError::InvalidInput(err) => write!(f, "invalid JSON input: {err}"),
            CliError::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for CliError {}

/// Reads and parses one JSON document from stdin. Used by every generated
/// command that takes a request body (`create`/`update`/`status`).
pub fn read_json_stdin<T: DeserializeOwned>() -> Result<T, CliError> {
    serde_json::from_reader(std::io::stdin()).map_err(CliError::InvalidInput)
}

/// Prints one JSON document to stdout, pretty-printed for readability while
/// remaining a single valid JSON document for automation to parse.
pub fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    let text = serde_json::to_string_pretty(value).map_err(CliError::InvalidInput)?;
    println!("{text}");
    Ok(())
}
