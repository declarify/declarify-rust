# Resource model

## Required protocol

The conceptual framework-owned envelope is:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeMeta {
    pub kind: String,
    #[serde(rename = "apiVersion")]
    pub api_version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
    pub id: String,
    pub name: Option<String>,
    pub labels: std::collections::BTreeMap<String, String>,
    pub annotations: std::collections::BTreeMap<String, String>,
    pub revision: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Resource<S, T> {
    #[serde(flatten)]
    pub type_meta: TypeMeta,
    pub metadata: Metadata,
    pub spec: S,
    pub status: T,
}
```

Applications own `S` and `T`. Declarify owns `TypeMeta`, `Metadata`, the
envelope, and the mutation rules.

An in-code resource declaration binds a concrete resource identity to the two
application types. Generated code exposes the public resource type, for example:

```rust
pub type Note = declarify_core::Resource<NoteSpec, NoteStatus>;
```

## Resource declaration semantics

Every resource declaration must provide, in Rust code:

- resource name/kind;
- API version;
- plural route/CLI name;
- `Spec` Rust type;
- `Status` Rust type;
- enabled operations.

The declaration must not repeat field definitions from `Spec` or `Status`.
Those fields exist only in the Rust types.

## Mutation semantics

- Create accepts permitted metadata inputs plus `Spec`.
  Declarify assigns ID, revision, timestamps, and static type metadata.
- Initial status is produced by `Default`. A custom initializer is deferred
  until a concrete scenario requires it.
- Replace-spec changes `Spec`, preserves `Status`, and increments revision.
- Update-status changes `Status`, preserves `Spec`, and increments revision.
- Replace-spec and update-status require an expected revision.
- A revision mismatch returns `conflict`.
- `kind`, `apiVersion`, ID, revision, and timestamps cannot be changed through
  ordinary request payloads.
- Delete removes the current resource. Delete history is a storage capability,
  not part of the resource contract.

## Supported field types

The code scanner supports only a deliberately narrow local Rust type subset:

- booleans, signed integers, floating-point numbers, and `String`;
- UUIDs and UTC timestamps through explicitly recognized canonical types;
- enums represented as stable strings;
- `Option<T>`;
- `Vec<T>`;
- string-keyed maps whose SQL representation is canonical JSON;
- nested local serializable structs stored as canonical JSON.

Declarify does not attempt general Rust type resolution. Type aliases, arbitrary
generics, custom serializers, flattened application fields, and unsupported
Serde transformations are declaration errors unless explicitly added to the
supported subset.

Unsupported constructs fail with diagnostics naming the resource, type/field,
and reason. Declarify never silently falls back to an opaque representation.

## Serialization rule

Application `Spec` and `Status` types must derive or implement `serde::Serialize`
and `serde::Deserialize`.

The generator assumes the supported Serde subset documented by the
scanner. The scanner must reject Serde attributes whose wire semantics it cannot
reproduce in generated SQL/API artifacts.

## Lists

List returns a typed page and uses an opaque cursor:

```rust
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
```

Declarify guarantees stable ordering by resource ID. Filtering beyond exact
metadata name or exact label matching is deferred.
