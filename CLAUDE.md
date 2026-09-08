# Working on Declarify

This repository is implementing Declarify Step 0.

Before changing code, read `README.md`, `IMPLEMENTATION.md`, and all documents
linked from the README Documentation section.

## Product invariants

- Resource definitions live in Rust code. Do not add a YAML/JSON resource
  manifest, schema authoring file, or second source of truth.
- `Spec` and `Status` are ordinary application-owned Rust types.
- `TypeMeta + Metadata + Spec + Status` is the required resource protocol.
- Step 0 is Rust only.
- Source generation is the primary mechanism; generated boundaries must remain
  readable source code.
- Declarify generates technical resource-management layers, not business logic.
- Declarify does not dogfood itself; `examples/notes` is the executable
  specification and TabulaScripta Knowledge is the first real consumer.

## Architecture rules

- keep domain, application, adapters, and composition dependencies separated;
- generated HTTP handlers depend on generated service traits;
- default services depend on repository traits, not SQL implementations;
- backend selection is handwritten composition;
- generated CLI calls the generated typed REST client, not repositories;
- generated code lives only in the generator-owned directory;
- never hand-edit generated files;
- never let regeneration overwrite handwritten files;
- do not hide consumer-specific ports/routes/repositories behind runtime magic
  merely to reduce generated line count.

## Scope discipline

Implement tickets from `docs/08-implementation-plan.md` in order.

Do not pull in OpenAPI, MCP, GraphQL, gRPC, Thrift, watch/events,
reconciliation, multi-language support, a public IR, plugin systems, auth, or
Dolt branch/history features unless the specification is explicitly revised.

When a required behavior is unclear:

1. check the existing specification;
2. prefer the smallest behavior needed by the current defining scenario;
3. record a real unresolved product decision in `docs/09-decisions.md`;
4. do not invent hidden semantics.

## Change workflow

For each ticket:

1. state the ticket and acceptance criterion being implemented;
2. add or update an executable defining scenario first when practical;
3. implement the smallest vertical behavior that satisfies it;
4. keep generated and handwritten ownership boundaries explicit;
5. update documentation only when behavior/decision actually changes;
6. run all required checks before declaring the ticket complete.

## Required checks before completion

At minimum run:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Also run the repository's generated-code drift check and Markdown link check
once those commands exist. For generated code, run compile fixtures and the
relevant real-server/CLI scenario for the current ticket.

A ticket is not complete because code generation succeeds. The observable
vertical scenario for that ticket must compile and execute.
