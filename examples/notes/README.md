# `notes`: the Declarify Step 0 tutorial

This is the executable specification for Declarify Step 0
(`../../README.md`, `../../IMPLEMENTATION.md`, `../../docs/`). It is also a
runnable tutorial: everything below is a real command you can run from a
fresh clone.

## Prerequisites

- Rust, pinned by `../../rust-toolchain.toml` (`rustup` installs it
  automatically the first time you build here).
- A C toolchain (needed to build the bundled SQLite/DoltLite engines —
  already present on macOS/Linux dev machines and CI images; on Windows, the
  MSVC Build Tools).
- `jq` and `curl`, only if you want to run `scripts/notes-lifecycle.sh`
  instead of `cargo test`.

All commands below assume your shell is in this directory
(`examples/notes`), except step 1.

## 1. Declare the resource (already done — read, don't run)

`src/domain/note.rs` is the entire authoring surface:

```rust
declarify::resource! {
    pub Note {
        api_version: "notes.declarify.dev/v1",
        plural: "notes",
        spec: NoteSpec,
        status: NoteStatus,
        operations: [create, get, list, update_spec, update_status, delete],
    }
}
```

There is no manifest anywhere else describing `Note`. If you change
`NoteSpec`/`NoteStatus` or the declaration above, that is the one place you
edit by hand.

## 2. Validate the declaration

From the repository root:

```console
$ cargo run -p declarify-cli -- validate --package notes
resource `Note` (notes.declarify.dev/v1, plural `notes`): spec `NoteSpec` (2 field(s)), status `NoteStatus` (1 field(s)), 6 operation(s)
declarify validate: 1 resource(s) valid, no manifest required
```

Try breaking something (e.g. add a `Vec<String>` field to `NoteSpec`, or
typo an operation name) and re-run — `validate` reports the exact
resource/type/field and reason, and never writes anything.

## 3. Generate

```console
$ cargo run -p declarify-cli -- generate --package notes
declarify generate: wrote 13 file(s) to .../examples/notes/src/generated
```

This is already committed in this repository (generated code is tracked
like ordinary source, not gitignored — see `docs/09-decisions.md`), so you
don't need to run it on a fresh clone. Running it again is safe: it
validates, renders to a temp directory, formats with `rustfmt`, compile-
checks the result with a real `cargo build`, and only then atomically
swaps it in — a failed run leaves the committed tree untouched. Confirm
there's no drift between what's committed and a fresh generation with:

```console
$ cargo run -p declarify-cli -- check --package notes
declarify check: generated tree matches a fresh generation
```

## 4. Build and test

```console
$ cargo build --workspace
$ cargo test --workspace
```

This runs the full lifecycle (create/get/list/replace-spec/update-status/
delete, revision conflicts, the handwritten title validator) three ways:
directly against the generated repository, over real HTTP through a
spawned server and the generated typed client, and through the compiled
CLI binary as a subprocess.

## 5. Run the real server

```console
$ cargo run --bin notes-server -- --db /tmp/notes.sqlite3 --listen 127.0.0.1:8080
notes-server listening on http://127.0.0.1:8080
```

(Leave this running in one terminal; use another for the next step.)

## 6. Drive it with the CLI

```console
$ echo '{"name":"first","spec":{"title":"Buy milk","body":"2%"}}' \
    | cargo run --bin notes -- create
$ cargo run --bin notes -- list
$ cargo run --bin notes -- get <id-from-above>
$ echo '{"title":"Buy oat milk","body":"2%"}' \
    | cargo run --bin notes -- update <id> --expected-revision 1
$ cargo run --bin notes -- delete <id> --expected-revision 2
```

`notes` defaults to `http://127.0.0.1:8080`; override with `--base-url` or
the `NOTES_BASE_URL` environment variable. Errors go to stderr with a
non-zero exit code.

Or run the whole thing as one script, which starts its own server and
cleans up after itself:

```console
$ ./scripts/notes-lifecycle.sh
```

## 7. Try the other storage backend (Linux/macOS only)

Everything above used SQLite (the default `sqlite-backend` Cargo feature).
The exact same generated repository, unmodified, also runs on DoltLite:

```console
$ cargo test -p notes --no-default-features --features doltlite-backend \
    --test note_lifecycle_repository
```

See `docs/09-decisions.md` for why this is a Cargo feature rather than a
second Rust type to choose between.

## Where things live

- `src/domain/` — handwritten: `NoteSpec`, `NoteStatus`, the resource
  declaration, the custom title validator.
- `src/generated/` — generator-owned. Never hand-edit; regenerate instead.
- `src/composition.rs` — handwritten: wires the generated service to a
  concrete repository, clock, ID generator, and validator.
- `src/bin/` — handwritten: the `notes` CLI and `notes-server` binaries.
- `tests/` — the conformance suite, run against every backend and every
  layer (repository, REST, CLI).
