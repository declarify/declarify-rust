# Storage

## Backend contract

Persistence is selected in handwritten composition. Domain and application
ports do not expose backend-specific connection types or version-control APIs.

There are two backends:

| Backend | Purpose | Platforms |
| --- | --- | --- |
| SQLite | first complete vertical path and portability baseline | Linux, macOS, Windows |
| DoltLite | primary experiment with future versioning potential | Linux, macOS |

SQLite is implemented first because it gives a small, portable baseline for the
repository contract. DoltLite must pass the same conformance suite before it is
considered usable.

The official DoltLite Rust crate currently vendors the engine, requires native
toolchain dependencies, and does not support Windows. This limitation is
isolated in the adapter and must not change resource/application contracts.

## SQL strategy

Generated repositories use a deliberately portable SQLite-compatible subset
shared by both backends.

Each resource has one table. Envelope fields use dedicated columns. Declarify
stores simple scalar fields as columns and nested/list/map values as canonical
JSON text. Generated column names use `spec_` and `status_` prefixes.

Every mutation is transactional and compares the expected revision in the SQL
predicate. Updating zero rows after an existing-ID check maps deterministically
to `conflict` or `not_found`.

## Adapter seam

The generated repository implements an application-facing trait. Connection
creation and backend selection remain handwritten.

A consumer can provide another repository implementation without changing
generated HTTP, client, or CLI contracts.

## Migrations

Declarify supports a fresh database generated from the current resource
configuration. General schema migration between arbitrary versions is deferred.

The generator may emit deterministic bootstrap SQL/migration files for the fresh
schema, but it must not pretend to solve production migration planning.

## Deferred DoltLite capabilities

Branches, commits, merges, diffs, history, remotes, and provenance are not
exposed by generic CRUD. They are introduced only after a consuming
application defines the required semantics.
