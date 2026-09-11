# Code Style

Engineering decisions start with an observable problem and a result that could disprove the proposed
explanation. Separate observed behavior from hypotheses, reproduce the relevant failure where
possible, and choose the smallest working change that addresses it. A complete explanation or a
passing build alone does not establish that the problem is fixed.

The primary organization rule is the feature boundary. Keep the types, reads, writes, protocol
handling, and focused tests for one feature together, while keeping transport and view layers thin.
A reviewer should be able to understand a change from the owning module, its contracts, and its
tests without loading unrelated features.

## Language boundaries

- Rust owns application behavior, external I/O, credentials, storage, parsing, concurrency, and
  reusable domain contracts.
- Content classification, domain filtering, and selecting canonical or latest projections belong in
  Rust; Svelte may group or format an already classified projection for display.
- Svelte owns rendering, user interaction, and small display-only transformations.
- TypeScript types mirror serialized Rust transport contracts; they do not become a second business
  model.
- Prefer English for application UI copy, labels, accessibility text, logs, error messages, and developer-facing names.
- Model transport absence as explicit `null`. Do not introduce `any`, `unknown`, optional fields,
  or `undefined` when a concrete serialized contract is available, and do not use `??` to hide an
  imprecise contract.
- Tauri commands translate between the frontend and Rust crates. They should not implement provider,
  UGOS, R2, or Markdown behavior.

## Packages and files

Organize code around the feature that owns its behavior and changes with it. Keep public contracts
narrow and dependencies explicit; directory moves alone do not reduce coupling. Create a package
only for a stable independent responsibility or genuinely shared code, and name modules after their
capabilities. Split a module when doing so isolates a real change or test boundary, not to meet a
line-count target.
Avoid generic `utils`, `helpers`, and `common` modules or moving a one-call-site function into a
shared module merely to make the caller shorter.

Desktop Svelte files distinguish page composition from supporting components. Under
`apps/desktop/src/lib/components`, `pages` owns complete navigation views and `layout` owns
cross-page shell controls, the shared page frame, and heading typography. Pages fill their
assigned content slot, centered within the main area by the shared frame. Shared page headings use semantic tokens; existing editorial and widget
compositions retain their internal proportions and typography. Unifying the outer frame must not
rearrange those compositions.
Supporting components and view sessions belong to the feature that owns them: `dashboard`, `games`,
`knowledge`, `memos`, `moment`, `settings`, or `inbox`. A feature session may hold its view cache,
request generations, event/timer cleanup, and typed command callbacks across page unmounts. Keep a
feature's reads, tags, and mutations together; do not introduce a shared content coordinator that
owns several features' mutable state. Rust still owns domain behavior and external I/O.
`App.svelte` composes the shell, creates feature sessions, and connects navigation, initial snapshots,
and configuration effects through narrow interfaces. Reusable UI primitives belong in `packages/ui`.
Tests follow the owning feature into `__tests__`; cross-feature navigation tests stay beside App.

Rust provider modules own their endpoint constants, wire types, parsing, request lifecycle, and
errors; AI usage providers remain separate files below `crates/useage/src`. Consumer modules keep
transport types, projections, and create, read, update, and delete operations together rather than
splitting directories by CRUD verb. Shared authentication belongs in `auth.rs` only when multiple
providers use the same credential format and resolution policy.

Stable capabilities retain their own boundaries: `r2.rs` owns object storage,
`cms-core::markdown` owns general Markdown compilation and article/Memo rendering; `md-dialect`
owns custom fence validation, provider data, and rendering. `build.rs` owns local artifact assembly.

## Helpers and abstractions

Start with the smallest implementation that meets the observed requirement. Let new abstractions
grow from demonstrated reuse or an invariant that needs one owner. Before adding a cache, retry
layer, configuration switch, or dependency, state what would fail or measurably worsen without it.
For optimization claims, compare with that element disabled while holding other variables fixed;
retain necessary correctness and safety constraints even if they do not improve a timing metric.

Add a helper when it:

- removes meaningful repeated logic from three or more call sites;
- isolates an external boundary such as HTTP, process I/O, credentials, R2, or UGOS;
- names a real domain operation; or
- centralizes an invariant, cleanup rule, or error policy.

Avoid helpers that only rename a single expression, wrap one method call, or hide control flow. Keep
simple transformations and explicit request flow next to the code that uses them.

Prefer compact names that retain the domain meaning. Do not encode an entire implementation or test
assertion in an identifier. Keep a longer externally mandated field name only at its wire boundary.

Rust's `?`, closures, and wildcard patterns are normal when their meaning is local and obvious. Do
not stack nested `Result` propagation as `??`, combine pattern binding into a long boolean chain, or
compress multi-rule validation into one expression. Name the intermediate result or use explicit
early returns. Do not create a one-call helper merely to avoid ordinary Rust syntax.

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

- Use tabs and double quotes; Vite Plus owns formatting and linting. The root and desktop Vite configs enforce tabs for TypeScript.
- Use `PascalCase` for components and types, `camelCase` for variables and functions, and concrete
  nouns for state.
- Define serialized command contracts in `apps/desktop/src/lib/consumer.ts`.
- Prefer inferred local types; add explicit types at component props, shared interfaces, and command
  boundaries.
- Keep provider states independent. Do not add one global loading boolean that erases settled cards.
- Prefer inferred types and explicit narrowing. Do not use type assertions, non-null assertions,
  or definite-assignment assertions to bypass missing validation or initialization.
- Use semantic CSS tokens and reusable UI primitives.
- Avoid frontend network access for application providers; invoke Rust commands instead.

## Naming

- Name database tables and credential accounts after the feature they own. The shared database is
  `vesper.sqlite3`; do not introduce feature JSON files or build-mode filename prefixes. Build
  configuration selects the credential backend.
- Functions use verbs: `read`, `refreshDashboard`, `saveR2Configuration`.
- Values use concrete nouns: `usage`, `subscription`, `snapshot`, `credentials`.
- Booleans describe predicates or state: `isAvailable`, `unlimitedQuota`, `refreshing`.
- Avoid vague module or value names such as `misc`, `manager`, `thing`, `payload`, or `temp` unless the
  protocol itself owns that term.
- Preserve externally mandated field names only in wire types. Serialize public Rust contracts to
  frontend `camelCase` explicitly.
- The workspace crate name `useage` is intentional. Provider modules use product names such as
  `opencode.rs` and `cherryin.rs`; do not encode plan variants into unrelated provider names.

## Tests

Tests belong to the feature that owns the behavior. Frontend tests live in its `__tests__` directory;
Rust unit bodies may live under `crates/*/tests/unit` or `apps/*/tests/unit` and be mounted from the
owning module when private access is required. Do not widen production APIs just to make tests
convenient.

For a behavioral fix, capture the trigger, expected result, and observed failure in the smallest
useful test or reproducible procedure. Run the same scenario before and after the change. Test
observable outcomes rather than restating implementation details, and include the failure mode the
change claims to prevent. Request lifecycles need representative delayed, failed, and out-of-order
responses, including navigation or credential changes where relevant.

Parse representative provider responses without credentials. Live authenticated tests remain
ignored and opt-in; automated tests must not depend on a developer's account balance or login.
Mocked success demonstrates local behavior, not availability or compatibility of a deployed service.
Scale testing to the change: a directory move needs import, discovery, and build verification, while
a concurrency fix needs evidence about response ordering.

## Review

For non-trivial behavior, concurrency, or boundary changes, use a reviewer who did not implement the
change when one is available. Give that reviewer the problem, acceptance criteria, and relevant
code and tests before the author's diagnosis. Author and reviewer record their initial conclusions
independently, then compare evidence. If independent review is unavailable, say so; a second pass by
the author is not independent review.

The reviewer actively seeks a counterexample: a failing input, request order, missing precondition,
or simpler implementation that invalidates the claim. Findings need a source location and a
reproducer or a clearly labeled untested scenario. Resolve disagreements with a discriminating test
or direct evidence rather than a vote or the order in which explanations were offered.

Review the final change against these questions:

- What observed problem does it solve, and does the same scenario distinguish before from after?
- Which claims are facts, which are hypotheses, and what evidence would overturn the conclusion?
- Can an added element be removed without losing required behavior or the measured benefit?
- Can the owning feature and its tests be understood without unrelated application context?
- Are errors explicit, settled data preserved, and stale responses prevented from overwriting newer
  state where requests can overlap?
- Do frontend contracts match Rust serialization, styles use semantic tokens, and provider outputs
  keep credentials private except for the trusted Settings prefill contract?
- Have relevant formatting, lint, checks, tests, and builds passed, and have the owning documents
  been rewritten to describe the final behavior?

Every non-trivial handoff states the evidence collected and its limits: what was not tested, what
could not be reproduced, and which conclusions remain assumptions. Name the next observation that
would resolve each material uncertainty. Report known evidence gaps that affect the conclusion or
release decision; if none were identified, state the review scope rather than inventing doubts.
