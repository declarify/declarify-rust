# Declaration and generation

## Authoritative declaration source

Step 0 resource definitions live entirely in Rust code.

There is no `declarify.yaml`, JSON manifest, external schema file, or second
authoring DSL.

Application-owned Rust types are authoritative for fields and serialization.
The Declarify in-code declaration is authoritative for resource identity and
exposure.

A representative Step 0 declaration is:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSpec {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

The declaration surface may be implemented by a macro or equivalent code marker,
but the user-facing rule is fixed: the declaration is part of the Rust source.

## Ownership of facts

| Fact | Authoritative source |
| --- | --- |
| resource kind/name | in-code Declarify declaration |
| API version | in-code Declarify declaration |
| plural name | in-code Declarify declaration |
| enabled operations | in-code Declarify declaration |
| Spec fields/types | Rust `Spec` type |
| Status fields/types | Rust `Status` type |
| serialization | supported Serde semantics on Rust types |
| generated SQL/REST/client/CLI | derived output only |

The same fact must never be configurable in two places.

## Scanner model

The external generator uses a constrained source scanner rather than a manifest.

The scanner:

1. identifies the selected Cargo package;
2. scans handwritten Rust source while excluding generated directories;
3. finds Declarify resource declarations;
4. resolves referenced local `Spec` and `Status` structs/enums;
5. validates the supported field and Serde subset;
6. builds an internal validated schema model used by all generators.

This internal schema model is an implementation detail, not an authoring format
and not a stable public IR in Step 0.

Parsing arbitrary Rust is not a product goal. Unsupported type-resolution cases
must fail with actionable diagnostics rather than trigger a larger abstraction.

## Commands

```text
declarify validate [--package <name>]
declarify generate [--package <name>]
declarify check [--package <name>]
```

- `validate` scans Rust declarations and reports all actionable errors without
  writing generated files.
- `generate` validates, renders to a temporary directory, formats the result,
  compile-checks required fixtures, and atomically replaces the generated tree.
- `check` exits non-zero when committed/generated output differs from a fresh
  generation.

No command accepts a resource manifest in Step 0.

## Determinism and safety

- identical handwritten Rust inputs and tool version produce byte-identical
  output;
- resources, fields, imports, routes, and schema objects use stable ordering;
- output contains no timestamp, host path, or nondeterministic map order;
- generated Rust is formatted with `rustfmt`;
- every generated file carries an ownership marker;
- the generator replaces only the configured generated directory;
- output outside that directory is never modified;
- a failed run leaves the previous generated tree untouched;
- a second generation produces no repository diff.

## Generated contract

For each resource, Step 0 generates:

- concrete resource alias and request/input types;
- repository trait;
- service trait and default CRUD service;
- SQL schema/migration and mappings;
- REST routes and handlers;
- typed Rust client methods;
- CLI subcommands;
- composition helpers accepting injected implementations.

Business rules, authorization, custom actions, workflow transitions, and product
orchestration are handwritten.
