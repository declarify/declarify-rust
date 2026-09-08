# Scenarios and quality

The `examples/notes` application is the executable specification.

## Defining scenarios

1. A developer defines `NoteSpec` and `NoteStatus` as ordinary Rust structs,
   declares `Note` in Rust code, runs `declarify validate`, and receives no
   manifest-related setup requirement.
2. `declarify generate` produces compiling Rust code without editing handwritten
   files.
3. A real server starts on SQLite. The generated CLI creates, reads, lists,
   replaces spec, updates status, and deletes a note through REST.
4. Replacing spec preserves status; updating status preserves spec; stale
   revisions are rejected through `If-Match`.
5. The same application selects DoltLite on supported platforms without
   regenerating domain, API, client, or CLI code and passes the same lifecycle
   tests.
6. The SQLite configuration compiles and runs on Linux, macOS, and Windows CI.
7. An invalid in-code declaration reports exact resource/type/field errors and
   leaves the generated tree unchanged.
8. Unsupported Rust/Serde constructs fail validation rather than being silently
   approximated.
9. Two consecutive generations are byte-identical and the second creates no Git
   diff.
10. A handwritten validator, service, repository test double, route, and CLI
    command compose with generated code without generated edits.
11. Removing or changing a resource declaration in Rust and regenerating updates
    only the generator-owned tree.

## Definition of done

- all defining scenarios are automated;
- `cargo fmt --check` passes;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes;
- `cargo test --workspace` passes;
- generated-code drift check passes;
- Markdown links resolve inside a clean checkout;
- generated code has compile tests in addition to golden-file tests;
- scanner validation has positive and negative fixtures;
- repository conformance tests run against SQLite and DoltLite where supported;
- REST tests use a real server and typed client;
- CLI smoke tests execute the compiled CLI against that server;
- the notes example documents a clean start from Rust declaration to working
  CRUD;
- CI covers Linux, macOS, and Windows, with DoltLite tests explicitly gated to
  supported platforms rather than silently skipped.

The framework is not complete merely because it can parse or render files. The
code-first generated vertical path must compile and execute.
