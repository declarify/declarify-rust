# Decisions and open questions

## Core decisions

| Topic | Decision |
| --- | --- |
| implementation language | Rust only |
| toolchain | stable Rust pinned in the repository |
| product mechanism | deterministic source generation |
| authoring model | code-first; resource definitions live entirely in Rust source |
| external manifest | none; YAML/JSON resource manifests are out of scope |
| resource declaration | Declarify-owned in-code declaration surface referencing application Rust types |
| application model | ordinary application-owned Rust `Spec` and `Status` types |
| resource protocol | required `TypeMeta + Metadata + Spec + Status` |
| schema extraction | constrained scan of Declarify declarations and supported local Rust types; not arbitrary Rust analysis |
| public IR | none currently; validated internal schema model is an implementation detail |
| architecture | domain, application, adapters, composition |
| operations | create/get/list/replace-spec/update-status/delete |
| initial status | `Status: Default` |
| concurrency | optimistic revision check on mutations |
| revision over HTTP | canonical `If-Match: "<revision>"`; no duplicate expected-revision JSON field |
| REST | generated JSON API using `axum` |
| client | generated typed async Rust client |
| CLI | generated `clap` commands backed by the REST client |
| first storage path | SQLite |
| experimental storage | DoltLite through the same application contract |
| cross-platform promise | core/tooling and SQLite on Linux/macOS/Windows; DoltLite on Linux/macOS |
| extension model | narrow traits and explicit composition; no plugin system |
| generated ownership | one replaceable generated directory |
| self-dogfooding | no |

## DCL-0001 implementation choices (recorded)

1. **Declaration syntax**: the proposed `declarify::resource!` function-like
   macro, exactly as shown in the README/IMPLEMENTATION examples. It is
   implemented in `declarify-macros` and re-exported from the `declarify`
   facade crate so consumers only ever depend on one crate name. The macro
   parses and validates the declaration at compile time (required fields
   present, `operations` drawn from `declarify_core::Operation`) but
   **expands to nothing** — it hides no application boundary and generates no
   consumer-visible code. This was chosen over an attribute/derive form
   because the body is not attached to a single item (it names two separate
   types, `spec` and `status`), which a derive macro cannot express cleanly.
2. **Scanner/macro duplication is deliberate**: `declarify-schema` does not
   reuse `declarify-macros`'s parser (a `proc-macro = true` crate cannot
   export ordinary library items to other crates), and does not expand the
   macro either — `declarify validate`/`generate` must work by scanning
   source text alone, without compiling the consumer crate. `declarify-schema`
   therefore keeps its own parser for the same declaration grammar, tested
   independently (`crates/declarify-macros/src/lib.rs` unit tests and
   `crates/declarify-schema/tests/fixtures.rs`), both driven by the single
   shared `declarify_core::Operation` vocabulary so the operation keyword list
   itself cannot drift between the two readings.
3. **Package/source discovery**: `declarify-schema::discovery` runs
   `cargo metadata --no-deps`, resolves `--package <name>` to that package's
   manifest directory, and recursively scans every `.rs` file under its
   `src/` directory, skipping any directory literally named `generated`. This
   avoids hardcoded paths and matches the eventual multi-package generated
   layout described in `docs/02-architecture.md`.
4. **Supported type subset**: only `String` and `bool` fields, exactly
   as scoped by `IMPLEMENTATION.md` step 7. A `Spec`/`Status` root type must
   be a local struct with named fields; a matching local item that is an enum,
   a tuple/unit struct, ambiguous (same name found in more than one scanned
   file), or missing entirely is a diagnostic, never a silent fallback. A
   `Spec`/`Status` type must derive both `serde::Serialize` and
   `serde::Deserialize`; any `#[serde(..)]` field attribute is rejected
   outright for now, since the scanner does not yet reproduce any Serde wire
   transformation. Every diagnostic names the resource, the type, the
   field (when applicable), and the reason, with a `file:line` location
   derived from `proc-macro2`'s `span-locations` feature.
5. **Crates chosen for DCL-0001**: `syn` 2.x (`full`, `extra-traits`) +
   `proc-macro2` (`span-locations`) + `quote` for declaration parsing and
   diagnostic rendering; `cargo_metadata` for package discovery; `clap`
   (derive) + `anyhow` for the CLI binary. No compile-fixture crate (e.g.
   `trybuild`) is introduced yet — DCL-0001 has nothing to compile-test beyond
   the `notes` example itself, which is exercised directly by
   `cargo test --workspace` and `cargo run -p declarify-cli -- validate`.
   Compile fixtures for *generated* code are DCL-0002 scope.

## DCL-0002 implementation choices (recorded)

1. **Domain module visibility convention**: generated code imports `Spec`/
   `Status` types by the exact file-based module path the scanner recorded
   (`crate::domain::note::NoteSpec`), not through any re-export. A consumer's
   domain submodule referenced by a resource declaration must therefore be
   declared `pub mod`, not `mod`. The scanner does not check this itself (it
   does not do full module-visibility resolution, consistent with "not a
   general Rust compiler front-end"); getting it wrong surfaces as an
   ordinary `E0603` from the real compile check inside `declarify generate`,
   not as a `declarify validate` diagnostic. `examples/notes` hit exactly
   this while DCL-0002 was implemented (`domain/mod.rs` originally had
   `mod note;`) and fixed it by making the submodule `pub`.
2. **Fixed `generated` module name and mount point**: the generated tree is
   always rooted at `src/generated/`, and a consumer wires it in with one
   handwritten line, `pub mod generated;`, in `src/lib.rs`. This is not
   configurable — a second name would be a second source of truth for
   something Declarify already fixes in `docs/02-architecture.md`'s consumer
   layout. This line (and the generated tree it points at) must exist before
   the very first `declarify generate` run compiles, so the generated tree is
   committed to version control like ordinary source, not gitignored.
3. **Generated code depends on `declarify-core`/`declarify-runtime`/`chrono`
   directly**, as ordinary Cargo dependencies the consumer's manifest lists
   alongside `declarify`. The `declarify` facade crate currently re-exports
   only the `resource!` macro. Consumers therefore add four dependencies, not
   one; this was chosen over widening the facade to re-export every framework
   type because generated code stays more directly readable at the consumer
   boundary (`declarify_core::Resource<..>` instead of an extra indirection
   like `declarify::core::Resource<..>`), matching
   `docs/02-architecture.md`'s "generated ports, routes, repositories, and
   client methods must remain readable at the consumer boundary."
4. **Repository vs. service layering**: the generated repository trait is a
   pure persistence port operating on an already-fully-formed resource or
   primitive field values (`insert(resource) -> Resource`, `replace_spec(id,
   expected_revision, spec, updated_at) -> Resource`, ...). It does not
   assign identity, timestamps, or the initial revision. The generated
   default service owns all of that (via the injected `Clock`/`IdGenerator`
   seams) plus the validator hook, before ever calling the repository. This
   follows `docs/01-resource-model.md` ("Create ... Declarify assigns ID,
   revision, timestamps") literally and keeps `docs/02-architecture.md`'s
   "default services depend on repository traits, not SQL implementations"
   rule meaningful: a repository implementation never needs Declarify's ID/
   clock policy to be correct.
5. **Generated repository/service traits are currently synchronous**, not
   `async fn`. `docs/05-rest-client-cli.md` specifies an async *typed client*
   (DCL-0004+), but says nothing about the application-layer traits, and
   `rusqlite` (the still-undecided-but-likely SQLite crate) is itself
   synchronous. Whether/how the DCL-0004 `axum` handlers bridge to this sync
   layer (direct call vs. `spawn_blocking`) is deferred to that ticket and
   must be recorded here before DCL-0004 starts.
6. **`declarify generate`'s exact sequence**, matching
   `docs/03-declaration-and-generation.md` word for word: validate (abort,
   untouched, on any diagnostic) -> render every file into a fresh
   `.generated.tmp` sibling directory -> `rustfmt` every rendered file in
   place -> atomically swap it in for the real `generated` directory via two
   renames (old one aside to `.generated.bak`, new one into place) -> run a
   real `cargo build -p <package>` as the compile check -> on success, drop
   the backup; on failure, rename the backup back and delete the bad tree, so
   a failed run leaves the previous generated tree byte-for-byte untouched.
   `declarify check` reuses the same render+format step into a throwaway
   temp directory and only ever *reads* the committed `generated` directory
   to diff against it (missing/stale/differing files), never writing to it.
7. **No compile-fixture crate (e.g. `trybuild`) yet**: the real compile check
   described above (a genuine `cargo build` of the consumer package with the
   freshly generated tree in place) already is DCL-0002's compile test, and
   `crates/declarify-codegen/src/lib.rs`'s unit tests cover determinism and
   the ownership-marker/file-set invariants directly on the in-memory
   `GeneratedTree`. Revisit only if a future ticket needs to compile-check
   generated code in isolation from a real consumer package.

## DCL-0003 implementation choices (recorded)

1. **SQLite crate: `rusqlite`**, with the `bundled` feature (statically
   compiles SQLite from vendored source, so Linux/macOS/Windows builds need
   no system SQLite package — only a C toolchain, which CI images already
   carry) and the `chrono` feature (`ToSql`/`FromSql` for
   `chrono::DateTime<Utc>`, so `created_at`/`updated_at` round-trip without
   hand-written conversions). It was chosen over alternatives (e.g. `sqlx`)
   because it is synchronous, matching the DCL-0002 decision that generated
   repository/service traits are not `async fn`, and its `Connection`/
   `Transaction` API maps directly onto "every mutation is transactional and
   compares the expected revision in the SQL predicate"
   (`docs/04-storage.md`).
2. **Concurrency model: one `rusqlite::Connection` behind a `std::sync::Mutex`
   per repository instance.** `rusqlite::Connection` is `Send` but not
   `Sync`; wrapping it in a `Mutex` is the smallest way to satisfy the
   repository trait's `Send + Sync` bound. This serializes all access to one
   resource's table through one repository instance within a process — an
   accepted prototype limit (`IMPLEMENTATION.md`: "a prototype with
   explicit extension boundaries, not a universal low-code platform"), not a
   production pooling story. A connection pool (e.g. `r2d2_sqlite`) is a
   candidate seam if a later scenario needs it, but nothing in the current
   defining scenarios does.
3. **`u64` revision stored as SQLite `INTEGER` via an explicit `as i64`/
   `as u64` cast at the repository boundary.** `rusqlite` does not implement
   `ToSql`/`FromSql` for `u64` (SQLite's native integer type is signed
   64-bit), so the generated code casts at read/write time rather than
   picking a different Rust-level revision type; `declarify_core::Metadata
   ::revision` stays `u64` because a revision counter is never meaningfully
   negative.
4. **Mutation pattern for `replace_spec`/`update_status`/`delete`**: a single
   `UPDATE ... WHERE id = ? AND revision = ?` (or `DELETE ...`) inside an
   explicit `rusqlite` transaction. Zero rows affected means either the ID
   does not exist or the revision was stale; a follow-up
   `SELECT EXISTS(...)` inside the *same* transaction disambiguates
   `NotFound` from `Conflict`, matching `docs/04-storage.md` exactly:
   "Updating zero rows after an existing-ID check maps deterministically to
   `conflict` or `not_found`." A successful mutation re-`SELECT`s the full
   row before committing, so the returned resource always reflects exactly
   what was persisted.
5. **List cursor is the literal last-returned resource ID**, used as a
   keyset-pagination boundary (`WHERE (?cursor IS NULL OR id > ?cursor)
   ORDER BY id ASC LIMIT ?`), not an encoded/opaque token. `docs/01-resource-
   model.md` calls the cursor "opaque" from the API's point of view (callers
   must treat it as a black box and pass it back verbatim, never construct or
   parse one) — Declarify does not additionally obfuscate it. Default page size
   when `PageRequest.limit` is `None`: 50.
6. **`labels`/`annotations` (`BTreeMap<String, String>`) are stored as
   canonical JSON text via `serde_json`**, per `docs/04-storage.md` ("nested/
   list/map values as canonical JSON text"). This is the first `serde_json`
   dependency in a consumer's `Cargo.toml`, alongside `rusqlite`.
7. **Table name is the resource's declared `plural`** (e.g. `notes`), one
   table per resource, columns named directly for envelope fields plus
   `spec_`/`status_`-prefixed columns per `Spec`/`Status` field in struct
   declaration order — exactly `docs/04-storage.md`'s "SQL strategy" section.

## DCL-0004 implementation choices (recorded)

1. **Sync-repository/async-HTTP bridging: call the synchronous service
   directly from the `async` handler, no `spawn_blocking`.** This was the
   open question from DCL-0002. Generated handlers depend on the
   `{Name}Service` trait, not on `Default{Name}Service`, so a consumer who
   needs true non-blocking dispatch under real concurrent load can supply a
   custom service implementation without touching generated code — the
   existing extension seam already covers it. For the current defining
   scenarios (a local dev tool, SQLite-backed, one client at a time in the
   tests), blocking the async task for the duration of a local SQLite call
   is an accepted prototype simplification, not a production concurrency
   story (`IMPLEMENTATION.md`: "a prototype with explicit extension
   boundaries").
2. **`axum` 0.8 path syntax**: routes use `{id}` (not the pre-0.8 `:id`).
   `axum::routing::MethodRouter::new()` is built up conditionally per
   resource so a route is only registered for methods the resource actually
   enables (e.g. a resource without `delete` gets no `DELETE` method on its
   `/{id}` route at all, rather than a handler that always errors).
3. **`Arc<dyn {Name}Service>` as the router's `axum` state**, not a generic
   `Router<S: {Name}Service>`. A trait object keeps the generated `routes()`
   function's signature simple and still satisfies "generated HTTP handlers
   depend on generated service traits, not the default service"
   (`CLAUDE.md`); the small dynamic-dispatch cost is irrelevant next to a
   network round trip.
4. **Shared HTTP building blocks live in `declarify-runtime::http`, not
   per-resource generated code**: `ApiError`/`ApiErrorBody` (the stable error
   envelope and its `declarify_core::Error` -> status/code mapping),
   `require_if_match` (the `If-Match: "<revision>"` parser), `ListQuery`, and
   the client-side `ClientError`/`parse_json_response`/`expect_no_content`
   helpers are resource-agnostic, so generating them per resource would be
   pure duplication — exactly what `docs/02-architecture.md`'s "small runtime
   crate may hold stable, type-independent building blocks ... shared HTTP
   helpers" describes. `declarify-runtime` now depends on `declarify-core`,
   `axum`, and `reqwest`.
5. **Typed client crate: `reqwest`** (`json` feature, `rustls-tls` instead of
   the system/OpenSSL-backed default, so no system TLS library is required
   on any of the three target platforms), paired with `tokio` in
   consumers for running async tests/servers. The client owns its own
   `reqwest::Client` (constructible via `new(base_url)` or
   `with_client(base_url, reqwest::Client)` for injected transport
   configuration, per `docs/05-rest-client-cli.md`) and never imports
   anything CLI-related.
6. **List query parameters are the `PageRequest` struct serialized directly**
   via `reqwest`'s `.query(&page)` (backed by `serde_urlencoded`), which
   omits `None` fields from the query string rather than emitting `limit=`/
   `cursor=` with empty values. No manual query-string building was needed.

## DCL-0005 implementation choices (recorded)

1. **CLI command contract**: `create` reads a full `Create<Name>Input` JSON
   document from stdin (it alone carries `name` alongside `spec`); `update`
   (replace-spec) and `status` (update-status) read the *bare* `Spec`/
   `Status` JSON document from stdin — not wrapped in `Replace<Name>SpecInput`
   /`Update<Name>StatusInput` — and take `--expected-revision <u64>` as an
   explicit flag; `delete` takes only `--expected-revision`. This was the
   open choice `docs/05-rest-client-cli.md` left between "a flag or read
   [the revision] from a supplied full resource document": a flag was
   chosen because it needs no special-case parsing of "a full resource vs.
   just its spec" and keeps stdin's shape uniform with what the command
   actually mutates.
2. **JSON output is always pretty-printed** (`serde_json::to_string_pretty`)
   to stdout, one document per invocation. Still a single valid JSON
   document per `docs/05-rest-client-cli.md` ("JSON input/output as the
   stable automation interface"); no table rendering was added (explicitly
   optional and out of scope per that doc).
3. **A single non-zero exit code (`1`) for any CLI failure** — transport,
   decode, or a structured API error alike — with the error's `Display`
   text on stderr. `docs/05-rest-client-cli.md` requires only "non-zero,"
   not a taxonomy of exit codes; a consumer needing to distinguish failure
   kinds already gets that from parsing stderr or, more robustly, from
   calling the typed client/REST API directly instead of shelling out.
4. **Shared CLI building blocks live in `declarify-runtime::cli`**
   (`CliError`, `read_json_stdin`, `print_json`), mirroring the DCL-0004
   `declarify-runtime::http` precedent: resource-agnostic JSON I/O has
   exactly one implementation, not one copy per generated command file.
5. **The generated `cli` module never defines a binary itself** — only a
   `#[derive(clap::Subcommand)] pub enum <Name>Command` plus an `async fn
   run(command, &<Name>Client)`. The consumer's `[[bin]]` entry point
   (`examples/notes/src/bin/notes.rs`) is handwritten: it owns `--base-url`
   parsing (`docs/05-rest-client-cli.md`: "accepts the server base URL
   explicitly and from configuration" — implemented as a `clap` flag with
   an `env = "NOTES_BASE_URL"` fallback) and the stderr/exit-code reporting
   in point 3. A second handwritten binary, `notes-server`, exists purely to
   bind the generated `axum` router to a real socket over the generated
   SQLite repository (`--db`/`--listen` flags, also env-configurable) — it
   is composition, not generated code, per `docs/06-extension-model.md`.
6. **Compiled CLI smoke test over a shell script as the primary check**:
   `tests/cli_smoke.rs` spawns a real in-process `axum`/SQLite server and
   drives the actual compiled `notes` binary
   (`env!("CARGO_BIN_EXE_notes")`) as a subprocess for the full lifecycle,
   satisfying `docs/07-scenarios-and-quality.md`'s DoD line verbatim ("CLI
   smoke tests execute the compiled CLI against that server") in a way that
   runs under plain `cargo test --workspace` with no external tools.
   `scripts/notes-lifecycle.sh` is the same walkthrough as a literal shell
   script against a separately-run `notes-server` process (satisfying the
   ticket's exit-criterion wording just as literally), kept as a
   human-facing artifact rather than a required check, since it depends on
   `jq`/`curl` being installed.

## DCL-0006 implementation choices (recorded)

1. **DoltLite crate: `rusqdoltlite`**, not the lower-level `doltlite`/
   `libdoltlite-sys` crates directly. `rusqdoltlite`'s own `Cargo.toml` sets
   `[lib] name = "rusqlite"` — it is a genuine drop-in replacement that
   compiles to a library registering under the **identical extern crate
   name** as real `rusqlite` (confirmed by inspecting its manifest and
   `src/lib.rs`: `Connection`, `Row`, `Transaction`, `params!`,
   `Error::QueryReturnedNoRows`, `Connection::transaction`/`open`/
   `open_in_memory`, and a `chrono` feature, all named identically to real
   `rusqlite`). This is exactly what `docs/04-storage.md` asks for
   ("Generated repositories use a deliberately portable SQLite-compatible
   subset shared by both backends"): **the generated persistence code for a
   resource did not need to change at all.** `declarify-codegen`'s
   `persistence.rs` still renders exactly one repository
   (`Sqlite<Name>Repository`, using `rusqlite::`), unchanged from DCL-0003.
2. **Consequence: backend selection is a Cargo feature, not a second Rust
   type.** Because `rusqlite` and `rusqdoltlite` link under the same extern
   name, they are mutually exclusive within one compiled crate — enabling
   both produces `error[E0464]: multiple candidates for rmeta dependency
   rusqlite`, confirmed empirically while implementing this ticket (see
   point 4). There is therefore no `DoltliteNoteRepository` type and no
   branch in `composition.rs` choosing between two repository constructors;
   `examples/notes/Cargo.toml` declares `rusqlite`/`rusqdoltlite` as
   mutually-exclusive optional dependencies behind `sqlite-backend`
   (default) / `doltlite-backend` features, and
   `composition::open_sqlite_note_repository` (name kept — it is, and
   remains, the "SQLite-API-shaped" repository regardless of which physical
   engine backs it) works unchanged under either. This still satisfies
   `docs/06-extension-model.md`'s "backend selection is handwritten
   composition" — the choice is made in a handwritten `Cargo.toml`, external
   to the generated tree — even though it is not the runtime/call-site choice
   the doc's phrasing might first suggest; this is recorded here precisely
   because the concrete mechanism turned out to differ from what DCL-0002's
   original repository-trait design implicitly assumed.
3. **DoltLite is gated to Linux/macOS** via `[target.'cfg(any(target_os =
   "linux", target_os = "macos"))'.dependencies]` for the optional
   `rusqdoltlite` dependency, matching the platform promise already
   recorded in this file's decisions table. `libdoltlite-sys` does expose an
   opt-in `bundled-windows` feature upstream (discovered while investigating
   the crate) — this project deliberately does not enable it, consistent with
   "Windows support for embedded DoltLite before upstream support exists"
   under "Explicitly deferred decisions" below.
4. **`cargo clippy`/`cargo test` with `--all-features` cannot be run for the
   `notes` package** — `--all-features` activates both `sqlite-backend` and
   `doltlite-backend` at once, hitting the exact E0464 collision from point
   2. This is a known, common shape for crates with mutually-exclusive
   backend features (`reqwest` itself has the same property across its TLS
   backends). The required checks are therefore run as three invocations
   instead of one blanket `--workspace --all-features` for this package:
   `cargo clippy --workspace --exclude notes --all-targets --all-features`,
   `cargo clippy -p notes --all-targets` (default = `sqlite-backend`), and
   `cargo clippy -p notes --all-targets --no-default-features --features
   doltlite-backend` — and the equivalent split for `cargo test`. Every
   integration test that exercises the repository directly, plus (found to
   also just work, unmodified) every REST and CLI test, was verified to pass
   under `doltlite-backend` this way.
5. **`tests/note_lifecycle_repository.rs`** (renamed from the DCL-0003
   `note_lifecycle_sqlite.rs`) is the one conformance suite for both
   backends — literally the same test source, run twice under different
   `--features`, per `docs/07-scenarios-and-quality.md` scenario 5 ("without
   regenerating domain, API, client, or CLI code and passes the same
   lifecycle tests"). No separate `note_lifecycle_doltlite.rs` was added,
   since there is no separate backend-specific code path left to exercise.

## DCL-0007 implementation choices (recorded)

1. **Tutorial location: `examples/notes/README.md`**, not a numbered doc
   under `docs/`. The numbered docs are explicitly "the complete
   specification" (root `README.md`); a runnable, command-by-command
   tutorial is a different kind of document and lives next to the code it
   walks through, in the conventional per-example-crate `README.md`
   location. The root `README.md` gained one link to it under a new
   "Getting started" section.
2. **Markdown link checker: a small `bash` script
   (`scripts/check-markdown-links.sh`)**, not an added tool/dependency
   (e.g. a `markdown-link-check` npm package, which would be the only
   reason for this Rust-only repository to need Node at all). It walks
   every `*.md` file, extracts Markdown links via `grep`/`sed`,
   skips absolute URLs and pure in-page anchors, and verifies the
   remaining relative targets exist on disk. It does not validate in-page
   anchors (`#section`) resolve to a real heading — a deliberately smaller
   check than a full tool would give, sufficient for this repository's
   actual link shape (all cross-file, none deep-linking into a specific
   heading).
3. **CI added now, closing the item deferred since DCL-0003**:
   `.github/workflows/ci.yml` runs the docs link check plus a
   Linux/macOS/Windows matrix covering exactly the required-checks command
   set now documented in `CLAUDE.md` (including the DCL-0006
   `--all-features`-incompatible three-way clippy split and two-way test
   split), with the `doltlite-backend` steps present but conditioned
   (`if: matrix.os != 'windows-latest'`) rather than omitted, per
   `docs/07-scenarios-and-quality.md`'s "explicitly gated ... rather than
   silently skipped." **This workflow has not itself been executed** — this
   development environment is macOS-only and has no way to run GitHub
   Actions. It was authored to run exactly the same commands already
   verified locally on macOS, but Linux/Windows execution is unverified
   until it actually runs in CI (i.e. after this is pushed to a GitHub
   remote). Flag any failure there as a real finding, not a rubber stamp.
4. **No version tag scheme existed yet**; this ticket's exit criterion (a
   downstream consumer can pin and consume Declarify) needs one to
   pin against. Chose a plain annotated git tag, `v0.1.0`, at this ticket's
   commit — matching `workspace.package.version` already set to `0.1.0` in
   the root `Cargo.toml` since DCL-0001. No crates are published anywhere
   (`publish = false` throughout); "pin" here means "pin this git ref/tag,"
   not a registry version.

## Post-handoff: first real CI run (resolved)

The workflow from point 3 above was pushed to `https://github.com/
declarify/declarify-rust` and actually ran. `ubuntu-latest` and
`macos-latest` passed outright. `windows-latest` failed, but not where
point 3 guessed (native `bundled` builds were fine) — it failed at
`declarify check`, reporting all 13 generated files as "differs from a
fresh generation."

**Root cause**: git's default `core.autocrlf` behavior on the Windows
runner converted the committed (LF) generated files to CRLF on checkout.
`declarify generate`'s fresh render still produces LF (our codegen writes
literal `\n`, and `rustfmt`'s default `newline_style` preserves that), so
every file's checked-out bytes differed from a fresh render by line
endings alone — not a real content drift. **Fix**: added a repository-root
`.gitattributes` with `* text=auto eol=lf`, forcing LF on checkout on every
platform regardless of the cloning machine's `core.autocrlf`. No generated
or handwritten file content needed to change; everything in the repository
was already LF-only (verified with a CRLF grep before adding the fix).

This is exactly the kind of gap `declarify check`'s own cross-platform
promise exists to catch. Confirmed closed: the CI run after the
`.gitattributes` fix (run 38342708865) passed on all three platforms —
`ubuntu-latest`, `macos-latest`, and `windows-latest`, including the
`declarify check` step that had failed before. No other issue surfaced.

## Post-handoff: license and release builds

The repository had `license = "MIT OR Apache-2.0"` in `Cargo.toml` since
DCL-0001 but never had a `LICENSE` file — nothing to actually license
anything. The user asked for Apache 2.0 specifically. Added a root
`LICENSE` with the canonical Apache License 2.0 text (fetched from
`apache.org`, not retyped, to avoid transcription errors in a legal
document; the only change from the canonical text is filling in the
Appendix's `Copyright [yyyy] [name of copyright owner]` placeholder with
`Copyright 2026 Declarify` — a reasonable default naming the GitHub
organization, flagged to the user as easy to change if wrong). Changed
`license` to the single SPDX identifier `"Apache-2.0"` to match — a project
can't truthfully claim a dual MIT/Apache-2.0 license while only shipping
one license's text, and the user's request didn't ask for MIT.

Also added `.github/workflows/release.yml`: builds `declarify`, `notes`,
and `notes-server` in release mode on all three platforms (triggered by
pushing a `v*` tag, or manually via `workflow_dispatch` with an existing
tag — needed to backfill releases for tags, like `v0.1.0`/`v0.1.1`,
pushed before this workflow existed), packages each platform's binaries
plus `README.md`/`LICENSE` into a `tar.gz` (Unix) or `.zip` (Windows)
named by the actual `rustc`-reported host triple (not a hardcoded
per-OS guess), and publishes them as GitHub Release assets.

## Still open (deferred)

- None. All defining scenarios pass, on all three target platforms, in
  real CI, as of run 38342708865.

## Post-handoff: GitHub Pages documentation site

Published the `docs/*.md` specification as a static site via `mdBook`, per
explicit request. Choices:

1. **`mdBook`, not Jekyll/MkDocs/a hand-rolled generator.** It is itself a
   Rust tool (`cargo install mdbook` or a prebuilt binary), matching this
   project's "one language throughout" stance better than a Node/Python
   toolchain would, and it's the standard choice for Rust-ecosystem
   documentation (`doc.rust-lang.org/book`, the Cargo Book).
2. **Site config lives in a dedicated `site/` directory (`site/book.toml`),
   not at the repository root**, per explicit request for the published
   site to have its own separate location. `book.toml` sets `src =
   "../docs"` so the *same* `docs/*.md` files already linked from
   `README.md` are the book's content — no second copy of the text to keep
   in sync. The one new file, `docs/SUMMARY.md` (mdBook's required table of
   contents), lives inside `docs/` itself, alongside the content it
   indexes, the same way `README.md`'s own "Documentation" list already
   does for humans.
3. **Build output (`site/book/`) is gitignored**, generated fresh by CI on
   every push to `main` and deployed directly via GitHub's
   `actions/deploy-pages`, never committed — the same reasoning already
   applied to `target/`: it's a build artifact, not a product deliverable
   (unlike `src/generated/`, which *is* committed, because generated
   *application* code is this project's actual product boundary, not
   incidental build output).
4. **Enabling Pages itself (repository Settings -> Pages -> Source: "GitHub
   Actions") is a manual, one-time step** that has not been done as of
   this commit — it's an account/repo-settings change, not something a PR
   merge does. The workflow will fail at the deploy step until that's set.

## Post-handoff: per-platform release packages

`.github/workflows/release.yml`'s per-OS "Package" steps now produce a
proper installer-style artifact per platform, not just a bare archive of
binaries, per explicit request:

- **Linux**: `.tar.gz`, now also containing `scripts/release/install.sh`.
- **macOS**: `.dmg` (via `hdiutil create -format UDZO`), not `.tar.gz` —
  the native macOS distribution format, also containing `install.sh`.
- **Windows**: `.zip` (unchanged format), now also containing
  `scripts/release/install.ps1`.

Both install scripts copy the three binaries onto a destination directory
and either print a PATH hint (`install.sh`: `/usr/local/bin` if writable
or running as root, else `~/.local/bin`) or add the destination to the
current user's PATH directly (`install.ps1`:
`%LOCALAPPDATA%\Declarify\bin`, via `[Environment]::SetEnvironmentVariable`
— the standard way to persist PATH changes on Windows without a reboot).
Both accept a `DECLARIFY_INSTALL_DIR` override. Neither requires elevated
privileges for the default per-user install path.

Verified locally before committing, not just reviewed: `shellcheck` on
`install.sh`; an actual `hdiutil create`/`hdiutil attach` round-trip
confirming the `.dmg` contains all expected files with correct
executable bits; a real `tar czf`/`tar tzvf` round-trip for the `.tar.gz`
path; `install.ps1` executed under a locally installed `pwsh` against
fake binaries in an isolated temp directory (both the fresh-install and
custom-`DECLARIFY_INSTALL_DIR` paths; the "already on PATH" branch could
not be exercised faithfully outside real Windows, since .NET's per-user
PATH persistence is Windows-registry-backed and doesn't carry the same
meaning under `pwsh` on macOS — reviewed by inspection instead). One real
bug was caught by this local testing and fixed before commit:
`install.sh` originally only `mkdir -p`'d the *fallback* install directory,
so a custom `DECLARIFY_INSTALL_DIR` pointing at a not-yet-existing (but
perfectly creatable) directory was incorrectly treated as unwritable and
silently redirected to `~/.local/bin`.

Do not create a general abstraction solely to keep either answer open.

## Explicitly deferred decisions

- stable/public Declarify IR;
- OpenAPI and other protocol generation;
- watch/event/reconciliation semantics;
- DoltLite branch/commit/diff/history semantics;
- schema migrations beyond a fresh database;
- authentication and authorization;
- APIs or generated clients in other languages;
- Windows support for embedded DoltLite before upstream support exists;
- runtime plugin loading;
- any specific downstream consumer's resource semantics.
