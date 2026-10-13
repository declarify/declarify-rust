# Extension model

Extensible means a consumer can replace defaults or add behavior at named
boundaries without editing generated code or forking Declarify.

It does not mean runtime plugin discovery.

## Required seams

| Need | Seam |
| --- | --- |
| domain validation | application validator trait/hook called by the service |
| business behavior | handwritten service implementing the generated service trait |
| persistence replacement | handwritten repository trait implementation |
| backend selection | handwritten composition chooses SQLite or DoltLite |
| identifier policy | injected ID generator |
| time policy | injected clock |
| additional HTTP behavior | handwritten router nesting, middleware, and routes |
| additional CLI behavior | handwritten commands composed with generated commands |
| client transport policy | injected client configuration/transport |

## Compatibility rules

- Generated handlers depend on service traits, not default services.
- Default services depend on repository traits, not SQL implementations.
- Constructors accept dependencies explicitly; no global registry is required.
- Public traits remain narrow and scenario-driven.
- A custom implementation is demonstrated in the example and conformance tests.
- Regeneration never requires merging handwritten and generated regions within a
  file.
- Resource declarations remain in handwritten Rust and are never generated back
  into the application's domain source.

## Non-goals

Declarify does not provide:

- a generic plugin ABI;
- dynamic loading;
- arbitrary interception chains;
- a generic hook bus;
- runtime registration of new resource schemas;
- generated business workflows.
