//! The DCL-0005 exit criterion: "a shell script can complete the resource
//! lifecycle using only the CLI" (docs/08-implementation-plan.md), verified
//! here as a compiled CLI smoke test against a real SQLite-backed server
//! (docs/07-scenarios-and-quality.md DoD: "CLI smoke tests execute the
//! compiled CLI against that server"). Drives the actual compiled `notes`
//! binary (`env!("CARGO_BIN_EXE_notes")`) as a subprocess — never calls
//! generated/application code directly — so this exercises the exact same
//! path a human running the CLI would.
//!
//! See `scripts/notes-lifecycle.sh` for the same walkthrough as a plain
//! shell script against a real, separately-run `notes-server` process.

mod support;

use std::io::Write;
use std::process::{Command, Output, Stdio};

use support::spawn_server;

fn run_cli(base_url: &str, args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_notes"));
    command
        .arg("--base-url")
        .arg(base_url)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .expect("spawn the compiled notes CLI binary");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .expect("child stdin is piped")
            .write_all(input.as_bytes())
            .expect("write CLI stdin");
    }
    drop(child.stdin.take());
    child
        .wait_with_output()
        .expect("wait for the notes CLI binary")
}

fn json_stdout(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "CLI exited non-zero; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "CLI stdout was not valid JSON ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn full_lifecycle_through_the_compiled_cli_binary() {
    let base_url = spawn_server().await;

    let created = json_stdout(&run_cli(
        &base_url,
        &["create"],
        Some(r#"{"name":"first","spec":{"title":"Title","body":"Body"}}"#),
    ));
    let id = created["metadata"]["id"].as_str().unwrap().to_string();
    assert_eq!(created["metadata"]["revision"], 1);
    assert_eq!(created["status"]["indexed"], false);

    let fetched = json_stdout(&run_cli(&base_url, &["get", &id], None));
    assert_eq!(fetched["spec"]["title"], "Title");

    let listed = json_stdout(&run_cli(&base_url, &["list"], None));
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);

    let updated = json_stdout(&run_cli(
        &base_url,
        &["update", &id, "--expected-revision", "1"],
        Some(r#"{"title":"New title","body":"Body"}"#),
    ));
    assert_eq!(updated["metadata"]["revision"], 2);
    assert_eq!(updated["spec"]["title"], "New title");
    assert_eq!(
        updated["status"]["indexed"], false,
        "update must preserve status"
    );

    let stale = run_cli(
        &base_url,
        &["update", &id, "--expected-revision", "1"],
        Some(r#"{"title":"Stale write","body":"Body"}"#),
    );
    assert!(!stale.status.success(), "stale update must fail");
    assert!(
        String::from_utf8_lossy(&stale.stderr).contains("conflict"),
        "stderr: {}",
        String::from_utf8_lossy(&stale.stderr)
    );

    let status_updated = json_stdout(&run_cli(
        &base_url,
        &["status", &id, "--expected-revision", "2"],
        Some(r#"{"indexed":true}"#),
    ));
    assert_eq!(status_updated["metadata"]["revision"], 3);
    assert_eq!(
        status_updated["spec"]["title"], "New title",
        "status update must preserve spec"
    );
    assert_eq!(status_updated["status"]["indexed"], true);

    let rejected = run_cli(
        &base_url,
        &["create"],
        Some(r#"{"name":null,"spec":{"title":"   ","body":""}}"#),
    );
    assert!(!rejected.status.success(), "blank title must be rejected");
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("invalid_argument"),
        "stderr: {}",
        String::from_utf8_lossy(&rejected.stderr)
    );

    let deleted = run_cli(
        &base_url,
        &["delete", &id, "--expected-revision", "3"],
        None,
    );
    assert!(
        deleted.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&deleted.stderr)
    );

    let after_delete = run_cli(&base_url, &["get", &id], None);
    assert!(!after_delete.status.success());
    assert!(
        String::from_utf8_lossy(&after_delete.stderr).contains("not_found"),
        "stderr: {}",
        String::from_utf8_lossy(&after_delete.stderr)
    );
}
