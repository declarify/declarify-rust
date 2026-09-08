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

## Still open (deferred)

- **SQLite crate choice** (`rusqlite` vs. alternatives) is not yet decided.
  It is out of scope until DCL-0003 ("SQLite vertical path") and must be
  recorded here before that ticket's persistence code is written.
- **Sync-repository/async-HTTP bridging strategy** (point 5 above) must be
  recorded here before DCL-0004 starts.

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
