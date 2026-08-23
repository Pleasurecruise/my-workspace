# Code Style

The primary organization rule is the feature boundary. Keep the types, reads, writes, protocol
handling, and focused tests for one feature together, while keeping transport and view layers thin.

## Language boundaries

- Rust owns application behavior, external I/O, credentials, storage, parsing, concurrency, and
  reusable domain contracts.
- Svelte owns rendering, user interaction, and small display-only transformations.
- TypeScript types mirror serialized Rust transport contracts; they do not become a second business
  model.
- Tauri commands translate between the frontend and Rust crates. They should not implement provider,
  UGOS, R2, or Markdown behavior.

## Packages and files

- Create a package only for a stable independent feature boundary or genuinely shared code.
- Keep provider implementations in separate files below `crates/useage/src`.
- A provider file owns its endpoint constants, wire types, parsing, request lifecycle, and errors.
- Keep a consumer's create, read, update, and delete operations in that consumer's feature file. Do
  not split files or directories by CRUD verb.
- Keep a consumer's transport types and projections in its feature module. Stable capabilities keep
  their own boundaries: `r2.rs` owns object storage, `markdown.rs` owns Markdown transformation, and
  `build.rs` owns local artifact assembly.
- Shared authentication belongs in `auth.rs` only when multiple providers use the same credential
  format and resolution policy.
- Avoid generic `utils`, `helpers`, or `common` modules. Name modules after the capability they own.
- Do not move a one-call-site function into a shared module merely to make the caller shorter.

## Helpers and abstractions

Add a helper when it:

- removes meaningful repeated logic from three or more call sites;
- isolates an external boundary such as HTTP, process I/O, credentials, R2, or UGOS;
- names a real domain operation; or
- centralizes an invariant, cleanup rule, or error policy.

Avoid helpers that only rename a single expression, wrap one method call, or hide control flow. Keep
simple transformations and explicit request flow next to the code that uses them.

## Error handling

- Prefer typed Rust errors for stable domain and infrastructure boundaries.
- Provider modules may return a focused user-facing `String` when the only consumer is a Tauri
  command and no caller needs to branch by variant.
- Use `Result`, `?`, `map_err`, pattern matching, and explicit response states.
- Do not add broad frontend `try/catch` blocks around normal command flows when Tauri already returns
  a tagged `ready` or `failed` response.
- Use a catch only when an actual exception boundary remains and the code can recover or add useful
  context.
- Never turn an error into empty data, zero balance, successful authentication, or another plausible
  business value.
- Error messages may name the provider and operation but must not expose credentials or sensitive
  response bodies.

## Rust

- Follow `cargo fmt` and keep Clippy clean with warnings denied.
- Use `snake_case` for modules, functions, variables, and fields; `PascalCase` for types and traits;
  `UPPER_SNAKE_CASE` for policy constants.
- Prefer concrete types and narrow visibility. Do not make internals public only for tests.
- Deserialize untrusted responses into provider-owned wire types before constructing public output.
- Bound external I/O with timeouts where a hung request or process would stall the application.
- Run independent I/O concurrently when partial ordering is not required.
- Do not hold a synchronous mutex guard across `.await`.
- Keep secrets out of `Debug`, provider response types, and tracing fields. The typed Settings
  response is the narrow exception for editing stored values in the local webview.

## TypeScript and Svelte

- Use tabs and double quotes; Vite Plus owns formatting and linting.
- Use `PascalCase` for components and types, `camelCase` for variables and functions, and concrete
  nouns for state.
- Define serialized command contracts in `apps/desktop/src/lib/consumer.ts`.
- Prefer inferred local types; add explicit types at component props, shared interfaces, and command
  boundaries.
- Keep provider states independent. Do not add one global loading boolean that erases settled cards.
- Use semantic CSS tokens and reusable UI primitives.
- Avoid frontend network access for application providers; invoke Rust commands instead.

## Naming

- Functions use verbs: `read`, `loadQuery`, `saveR2Configuration`.
- Values use concrete nouns: `usage`, `subscription`, `snapshot`, `credentials`.
- Booleans describe predicates or state: `isAvailable`, `unlimitedQuota`, `refreshing`.
- Avoid vague module or value names such as `misc`, `manager`, `thing`, `payload`, or `temp` unless the
  protocol itself owns that term.
- Preserve externally mandated field names only in wire types. Serialize public Rust contracts to
  frontend `camelCase` explicitly.
- The workspace crate name `useage` is intentional. Provider modules use product names such as
  `opencode.rs` and `cherryin.rs`; do not encode plan variants into unrelated provider names.

## Tests

- Tests belong to the crate or package that owns the behavior.
- Rust unit bodies may live under `crates/*/tests/unit` or `apps/*/tests/unit` and be mounted from the
  owning module when private access is required.
- Test wire parsing with representative provider responses.
- Authenticated provider tests remain ignored and opt-in; automated tests must not depend on a
  developer's account balance or login.
- UI behavior should be tested at the smallest useful component or state boundary.
- Do not widen a production API solely to make a test convenient.

## Review checklist

- Is code grouped by feature rather than by CRUD verb or generic technical responsibility?
- Is new shared code justified by stable reuse or a real boundary?
- Are errors explicit without broad catches or misleading fallbacks?
- Are provider credentials and sensitive response bodies absent from logs and UI transport, apart
  from the documented Settings prefill response?
- Do frontend contracts match Rust serialization?
- Does loading remain independent and preserve settled data?
- Are semantic design tokens used throughout the UI?
- Are parsing, formatting, Clippy, checks, tests, and relevant builds passing?
- Were the owning documents updated without expanding the root README?
