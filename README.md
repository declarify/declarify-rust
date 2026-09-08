# Declarify

Declarify is a small, opinionated Rust framework and deterministic source
generator for resource-oriented applications.

The source of truth is application Rust code. Applications define ordinary
Rust `Spec` and `Status` types and declare resources in Rust with a
Declarify-owned code declaration. Declarify generates the repetitive outer
layers around those application-owned types: resource envelopes, application
ports and default CRUD services, SQL persistence, REST API, typed Rust client,
CLI commands, and composition glue.

The required resource protocol is:

`TypeMeta + Metadata + Spec + Status`

Step 0 has one purpose: provide the smallest usable foundation for
TabulaScripta Knowledge. It is a prototype with explicit extension boundaries,
not a universal low-code platform and not a general control plane.

## Non-negotiable code-first rule

There is no YAML/JSON resource manifest in Step 0.

- resource identity and enabled operations are declared in Rust code;
- `Spec` and `Status` fields are ordinary application-owned Rust types;
- generated schemas and interfaces are derived from those Rust declarations;
- the same resource fact must never be configurable in a second source of truth.

The initial declaration shape is intentionally small:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteSpec {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NoteStatus {
    pub indexed: bool,
}

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

The exact parser and macro mechanics are proven in DCL-0001, but the product
semantics above are already decided: resource declarations live in Rust code.

## Step 0 result

A Rust application can:

1. define `NoteSpec` and `NoteStatus` in ordinary Rust;
2. declare `Note` in Rust code;
3. validate the declaration and generate deterministic Rust code;
4. persist resources through SQLite on Linux/macOS/Windows;
5. use DoltLite through the same storage boundary on Linux/macOS;
6. expose REST CRUD and a separate status operation;
7. call the API through a generated typed Rust client and generated CLI;
8. replace generated defaults through explicit traits/composition without
   editing generated files.

Declarify does not contain TabulaScripta concepts and does not dogfood itself.

## Documentation

- [Vision and scope](docs/00-vision-and-scope.md)
- [Resource model](docs/01-resource-model.md)
- [Architecture](docs/02-architecture.md)
- [Declaration and generation](docs/03-declaration-and-generation.md)
- [Storage](docs/04-storage.md)
- [REST API, client, and CLI](docs/05-rest-client-cli.md)
- [Extension model](docs/06-extension-model.md)
- [Scenarios and quality](docs/07-scenarios-and-quality.md)
- [Implementation plan](docs/08-implementation-plan.md)
- [Decisions and open questions](docs/09-decisions.md)
- [Implementation handoff](IMPLEMENTATION.md)

These documents are the complete Step 0 specification. Every relative link in
this list must resolve in CI.

## Technology baseline

- Rust stable, pinned by `rust-toolchain.toml`;
- Cargo workspace;
- `serde` for resource serialization;
- `syn`/Rust-source scanning only for the deliberately constrained Declarify
  declaration surface and supported local data types;
- `axum` for generated HTTP adapters;
- `clap` for generated CLI commands;
- a narrow Declarify-owned SQL port with SQLite and DoltLite adapters.

Exact dependency versions are chosen and locked during DCL-0001 bootstrap.
