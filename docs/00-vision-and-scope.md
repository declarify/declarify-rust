# Vision and scope

## Product statement

Declarify generates the resource-management shell around application-owned Rust
domain types.

Applications write ordinary Rust `Spec` and `Status` types and declare which
pairs form managed resources. Declarify removes mechanical repetition around
those resources while leaving product meaning and business behavior in ordinary
handwritten Rust.

The central opinion is that every managed object has four parts:

- `TypeMeta`: framework-owned resource kind and API version;
- `Metadata`: framework-owned identity and lifecycle information;
- `Spec`: application-owned desired or declared state;
- `Status`: application-owned observed or computed state.

Declarify generates technical boundaries around that protocol. It does not
generate product meaning.

## Code-first source of truth

Step 0 is code-first with one authoritative source: Rust code.

- `Spec` and `Status` structure is defined by Rust types.
- Resource identity, plural name, API version, and enabled operations are
  declared in Rust through the Declarify declaration surface.
- Generated SQL, REST routes, client methods, CLI commands, and schemas derive
  from those code declarations.
- Step 0 has no resource YAML/JSON manifest and no parallel schema DSL.

A generator may use a deliberately constrained Rust scanner internally. That is
an implementation mechanism, not a second authoring model.

## Why Rust

Step 0 uses one language for the generator, framework libraries, server,
generated client, CLI, tests, and consuming application: Rust.

Rust fits because Declarify is a local developer tool and an application
foundation:

- native binaries for Linux, macOS, and Windows;
- explicit types for generated contracts;
- predictable deployment without a language runtime;
- strong compile-time verification of generated code;
- procedural macros and deterministic source generation can coexist without
  introducing a second implementation language.

Step 0 uses source generation as the primary product mechanism. A small macro or
marker may define the resource declaration surface, but generated application
boundaries remain concrete source files and must stay readable.

## In scope

- ordinary Rust `Spec` and `Status` types;
- in-code declaration of those types as resources;
- required `TypeMeta + Metadata + Spec + Status` protocol;
- create, get, list, replace spec, update status, and delete;
- optimistic concurrency through metadata revision;
- deterministic generation of resource/application code;
- SQL persistence through a narrow backend boundary;
- SQLite as the first complete vertical persistence path;
- DoltLite as the primary experimental backend after SQLite conformance works;
- REST server adapter, typed Rust REST client, and CLI;
- deterministic and safe regeneration;
- explicit replacement and extension seams;
- one complete executable example application.

## Out of scope

- YAML/JSON resource manifests;
- runtime schema authoring;
- graphical UI;
- authentication and authorization;
- multi-tenancy;
- arbitrary queries and generic workflow engines;
- watch/event/reconciliation frameworks;
- event sourcing;
- database branches, merges, remote sync, or semantic diff as public framework
  features;
- OpenAPI/GraphQL/gRPC/MCP as Step 0 deliverables;
- generators or runtimes for languages other than Rust;
- runtime plugin discovery or dynamic loading;
- TabulaScripta domain objects;
- self-hosting or self-dogfooding.

DoltLite-specific versioning features are intentionally deferred until a real
TabulaScripta scenario requires them.
