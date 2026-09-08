# Decisions and open questions

## Step 0 decisions

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
| public IR | none in Step 0; validated internal schema model is an implementation detail |
| architecture | domain, application, adapters, composition |
| operations | create/get/list/replace-spec/update-status/delete |
| initial status | `Status: Default` in Step 0 |
| concurrency | optimistic revision check on mutations |
| revision over HTTP | canonical `If-Match: "<revision>"`; no duplicate expected-revision JSON field |
| REST | generated JSON API using `axum` |
| client | generated typed async Rust client |
| CLI | generated `clap` commands backed by the REST client |
| first storage path | SQLite |
| experimental storage | DoltLite through the same application contract |
| cross-platform promise | core/tooling and SQLite on Linux/macOS/Windows; DoltLite on Linux/macOS in Step 0 |
| extension model | narrow traits and explicit composition; no plugin system |
| generated ownership | one replaceable generated directory |
| first consumer | TabulaScripta Knowledge after notes handoff |
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
4. **Supported Step 0 type subset**: only `String` and `bool` fields, exactly
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
5. **Generated repository/service traits are synchronous for Step 0**, not
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
   accepted Step 0 prototype limit (`IMPLEMENTATION.md`: "a prototype with
   explicit extension boundaries, not a universal low-code platform"), not a
   production pooling story. A connection pool (e.g. `r2d2_sqlite`) is a
   candidate seam if a later scenario needs it, but nothing in Step 0's
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
   parse one) — Step 0 does not additionally obfuscate it. Default page size
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
   existing extension seam already covers it. For Step 0's defining
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
   on any of Step 0's three target platforms), paired with `tokio` in
   consumers for running async tests/servers. The client owns its own
   `reqwest::Client` (constructible via `new(base_url)` or
   `with_client(base_url, reqwest::Client)` for injected transport
   configuration, per `docs/05-rest-client-cli.md`) and never imports
   anything CLI-related.
6. **List query parameters are the `PageRequest` struct serialized directly**
   via `reqwest`'s `.query(&page)` (backed by `serde_urlencoded`), which
   omits `None` fields from the query string rather than emitting `limit=`/
   `cursor=` with empty values. No manual query-string building was needed.

## Still open (deferred)

- **Cross-platform CI** (`docs/07-scenarios-and-quality.md`: "CI covers
  Linux, macOS, and Windows") has not been set up yet; DCL-0003/DCL-0004 have
  only been run and verified on macOS (this development environment) via
  `cargo test --workspace`. No implementation-plan ticket has yet asked for
  CI configuration; revisit at latest by DCL-0007 ("Consumer handoff").

Do not create a general abstraction solely to keep either answer open.

## Explicitly deferred decisions

- stable/public Declarify IR;
- OpenAPI and other protocol generation;
- watch/event/reconciliation semantics;
- DoltLite branch/commit/diff/history semantics;
- schema migrations beyond a fresh Step 0 database;
- authentication and authorization;
- APIs or generated clients in other languages;
- Windows support for embedded DoltLite before upstream support exists;
- runtime plugin loading;
- TabulaScripta-specific resource semantics.
