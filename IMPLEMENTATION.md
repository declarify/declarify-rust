# Declarify — implementation handoff

This file is the execution entry point for Claude Code.

## Goal

Build the smallest reliable code-first Rust framework that lets a consuming
application define resources in Rust and obtain a working vertical stack:

```text
Rust Spec/Status + in-code resource declaration
                ↓
          validate / scan
                ↓
      deterministic generation
                ↓
 application ports + default CRUD
                ↓
       SQLite repository
                ↓
        generated REST API
                ↓
       typed Rust REST client
                ↓
         generated CLI
                ↓
     DoltLite adapter (optional backend)
```

The first executable resource is `Note`.

## Canonical declaration

Start DCL-0001 from this exact semantic shape:

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

If an attribute/derive syntax proves materially simpler than `resource!`, it may
replace this surface during DCL-0001, but only if all resource identity and
operation metadata remain in Rust code and the fields remain owned solely by the
Rust `Spec`/`Status` types.

## Recommended first repository shape

```text
crates/
  declarify-core/
  declarify-macros/
  declarify-schema/
  declarify-codegen/
  declarify-cli/
  declarify-runtime/
examples/
  notes/
docs/
testdata/
CLAUDE.md
README.md
```

Do not split into language-specific repositories.

## DCL-0001 execution order

1. Bootstrap the workspace and pin stable Rust.
2. Add `examples/notes` with only handwritten `NoteSpec`, `NoteStatus`, and the
   in-code resource declaration.
3. Make the declaration valid Rust with the smallest macro/marker crate.
4. Implement source discovery for one Cargo package.
5. Parse the declaration and build a small internal schema representation.
6. Resolve `NoteSpec` and `NoteStatus` from handwritten local Rust source.
7. Support only `String` and `bool` initially if that is sufficient for the
   first fixture; widen only through explicit tests.
8. Implement `declarify validate`.
9. Add negative fixtures for missing referenced types, duplicate resource names,
   unsupported field types, and unsupported Serde behavior.
10. Record the exact chosen syntax/scanner limits in `docs/09-decisions.md`.

Stop after the DCL-0001 exit criterion. Do not start CRUD code generation before
validation is reliable.

## Definition of DCL-0001 success

From a clean checkout, a developer can run a command equivalent to:

```text
cargo run -p declarify-cli -- validate --package notes
```

and Declarify discovers the `Note` resource from Rust source without any
manifest.

Changing `NoteSpec` changes what Declarify validates. Removing the declaration
removes the resource. There is no second resource description to keep in sync.

## What to optimize for

Prefer:

- small explicit contracts;
- deterministic output;
- compile-time and executable verification;
- readable generated code;
- actionable diagnostics;
- replacement through traits and composition;
- a working end-to-end vertical path.

Do not optimize yet for:

- arbitrary Rust syntax;
- maximal generator reuse;
- multi-language architecture;
- protocol breadth;
- plugin ecosystems;
- clever macro metaprogramming;
- Dolt-specific product features.

## Completion protocol for Claude Code

At the end of every DCL ticket, report:

1. implemented acceptance criterion;
2. files/crates added or materially changed;
3. tests added;
4. exact commands run and their result;
5. unresolved decision, if any;
6. why the next ticket is now safe to start.
