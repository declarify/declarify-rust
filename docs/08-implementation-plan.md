# Implementation plan

Each increment ends with a compiling workspace and one observable capability.
Claude Code should implement these tickets in order and must not begin a later
ticket to compensate for an unresolved earlier one.

## DCL-0001 — Code-first Rust declaration spike

Goals:

- bootstrap the Cargo workspace and pinned stable toolchain;
- add the minimal crates needed for `core`, declaration marker/macro, scanner,
  codegen shell, and CLI;
- define `NoteSpec` and `NoteStatus` in `examples/notes`;
- declare `Note` in Rust code using the Step 0 Declarify declaration surface;
- implement a constrained Rust scanner that finds resource declarations and
  resolves the referenced local `Spec`/`Status` shapes;
- support only the initial field subset needed by the notes fixture;
- implement `declarify validate` with precise diagnostics;
- add negative fixtures for unsupported/ambiguous types;
- record exact supported syntax and any scanner restriction in
  `docs/09-decisions.md`.

Non-negotiable acceptance criterion:

> A valid resource is fully authorable from Rust source alone. No YAML/JSON
> manifest or external resource schema is required or accepted.

Exit: `declarify validate` successfully resolves the notes declaration from Rust
code; invalid declarations fail clearly; nothing beyond validation needs to be
generated yet.

## DCL-0002 — Resource and application generation

- implement framework-owned envelope types and shared errors;
- generate concrete resource aliases and request/input types;
- generate repository trait, service trait, and default CRUD service;
- generate explicit composition helpers;
- implement safe deterministic output and `declarify check`;
- add golden, compile, and regeneration tests;
- demonstrate a handwritten validator or custom service implementation.

Exit: generated resource/application code compiles; generation is deterministic;
custom application behavior is injected without generated edits.

## DCL-0003 — SQLite vertical path

- choose and lock the SQLite crate;
- generate fresh-schema SQL and mappings for the supported field subset;
- implement create/get/list/replace-spec/update-status/delete;
- use optimistic revision checks transactionally;
- implement stable ordering and opaque list cursor;
- run repository lifecycle and conflict tests.

Exit: the full resource lifecycle passes directly through SQLite on Linux,
macOS, and Windows CI.

## DCL-0004 — REST and typed client

- generate `axum` routes and handlers;
- implement the stable JSON and error contracts;
- implement `If-Match` revision semantics exactly as specified;
- generate typed async Rust client methods;
- test through a real local server backed by SQLite.

Exit: the complete lifecycle passes through REST using only the generated typed
client.

## DCL-0005 — CLI

- generate `clap` commands using the REST client;
- implement JSON I/O and exit-code behavior;
- provide base URL configuration;
- add compiled CLI smoke tests against the real SQLite-backed server.

Exit: a shell script can complete the resource lifecycle using only the CLI.

## DCL-0006 — DoltLite adapter

- integrate the official DoltLite crate on Linux/macOS;
- keep the application-facing repository contract unchanged;
- run the same repository conformance suite used for SQLite;
- verify backend switching occurs only in handwritten composition;
- document native build prerequisites and upstream platform limitations.

Exit: the notes example can select DoltLite on supported platforms and pass the
same lifecycle tests.

## DCL-0007 — Consumer handoff

- finish the notes tutorial;
- verify a clean clone, validate, generate, build, server run, and CLI lifecycle;
- verify all Markdown links;
- verify generated drift checks;
- tag the minimal version consumable by TabulaScripta Knowledge.

Exit: TabulaScripta Knowledge can pin and consume Declarify without access to
prior project context.

## Do not pull forward

The following are explicitly not reasons to widen a ticket:

- OpenAPI generation;
- watch/events;
- MCP;
- GraphQL/gRPC/Thrift;
- multi-language support;
- general public IR;
- plugin systems;
- Dolt branches/commits/history;
- authentication/authorization;
- general schema migration.
