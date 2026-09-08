# REST API, client, and CLI

## REST contract

For `Note` declared with plural `notes`:

| Operation | Method | Path | Success |
| --- | --- | --- | --- |
| Create | `POST` | `/api/v1/notes` | `201` + resource |
| Get | `GET` | `/api/v1/notes/{id}` | `200` + resource |
| List | `GET` | `/api/v1/notes` | `200` + page |
| Replace spec | `PUT` | `/api/v1/notes/{id}` | `200` + resource |
| Update status | `PUT` | `/api/v1/notes/{id}/status` | `200` + resource |
| Delete | `DELETE` | `/api/v1/notes/{id}` | `204` |

Create and replace-spec payloads cannot write status. Status payloads cannot
write spec.

## Revision transport

Step 0 uses the HTTP `If-Match` header as the canonical optimistic-concurrency
precondition for replace-spec, update-status, and delete.

The value is the decimal resource revision quoted as an entity tag, for example:

```text
If-Match: "7"
```

- missing `If-Match` on a mutating existing-resource operation returns
  `invalid_argument`;
- a mismatched revision returns `conflict`;
- the current resource representation contains `metadata.revision`;
- Step 0 does not duplicate the expected revision in request JSON.

## Errors

Errors have one stable JSON envelope with:

- machine-readable `code`;
- human-readable `message`;
- optional structured `details`.

Required codes are:

- `invalid_argument`;
- `not_found`;
- `conflict`;
- `internal`.

HTTP status mapping must be deterministic and tested.

## Typed Rust client

The generated async client:

- accepts injected base URL and HTTP transport/configuration;
- exposes one typed method per enabled operation;
- returns typed resources and pages;
- automatically sends `If-Match` for mutation methods when given an expected
  revision;
- preserves structured API errors separately from transport and decoding errors;
- does not depend on CLI code.

## CLI

Generated `clap` subcommands call the REST client rather than repositories:

```text
notes create
notes get <id>
notes list
notes update <id>
notes status <id>
notes delete <id>
```

Step 0 supports JSON input/output as the stable automation interface. Friendly
table rendering is optional and must not become a blocker.

Mutation commands require an expected revision flag or read it from a supplied
full resource document according to the generated command contract. The CLI
must ultimately call the same typed client method and therefore the same REST
contract.

Errors go to stderr and produce non-zero exit codes. The CLI accepts the server
base URL explicitly and from configuration.
