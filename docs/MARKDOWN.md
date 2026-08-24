# Markdown Pipeline

Vesper compiles Markdown in Rust. The Svelte layer receives rendered HTML and display metadata; it
does not own parsing rules. This keeps Desktop and CLI behavior aligned and prevents consumer-specific
frontend parsers from producing different output for the same Markdown source.

## Current pipeline

```text
API or local Markdown
  -> cms-core::markdown
  -> pulldown-cmark events
  -> HTML + table of contents + excerpt
  -> typed Tauri response
  -> Svelte presentation
```

`render_memo` converts soft line breaks into hard line breaks to preserve the compact writing style
used by my-memos. `compile_knowledge` assigns stable, de-duplicated heading IDs and produces the table
of contents and excerpt in the same pass boundary as HTML compilation. Consumers continue to own
storage and metadata; Vesper does not retain a second Markdown mirror.

## What Waku does

[Waku][waku] has two Markdown surfaces with different constraints:

- Its native GPUI application parses with `pulldown-cmark` into a typed block tree. Every top-level
  block retains its byte range, and inline formatting is represented as styled text runs rather than
  serialized HTML.
- Its web application uses `react-markdown` with `remark-gfm`. A small rehype plugin wraps only newly
  appended text ranges to animate streaming output without replaying animation on stable content.

The native parser enables tables, strikethrough, and task lists. It treats raw HTML as literal text,
which is a deliberate transcript-safety decision. Its renderer maps paragraphs, headings, images,
code, quotes, lists, tables, and rules directly to GPUI elements.

### Incremental parsing

Waku's `IncrementalParser` recognizes append-only changes and reparses from the last stable source
boundary. It deliberately keeps the last two top-level source groups unsettled because an appended
table row or an inline image can change the preceding group. Link-reference definitions force a full
reparse because they can resolve references anywhere in the document.

For a still-streaming tail, Waku builds a display-only repaired version. It temporarily closes
unfinished emphasis, code spans, strikethrough, and links so formatting does not jump when the final
delimiter arrives. The canonical source and canonical parse tree are never modified.

### Stable rendering

Waku shapes one text element per block and applies inline style runs over the flat text. Syntax color,
inline-code backgrounds, search highlights, and selections are paint operations that do not alter
layout geometry. Settled blocks and tokenized code are cached, so an append rebuilds only volatile
tail content. Its syntax highlighter is a lightweight internal line tokenizer with cross-line state,
not a general-purpose compiler.

## Vesper adoption boundary

Vesper's stored Memo and Knowledge documents are settled content, so the current full
`pulldown-cmark` compilation remains the appropriate path. Introducing a second AST or a frontend
Markdown dependency now would add two sources of rendering truth without improving stored-document
behavior.

If Vesper later adds a live streaming preview, adopt the reusable ideas rather than Waku's GPUI
renderer:

1. Keep the canonical Markdown untouched and create display-only repairs for incomplete syntax.
2. Preserve source byte ranges in an intermediate representation.
3. Reparse only an append-only volatile tail, with a full-parse fallback for non-local constructs.
4. Keep syntax color and reveal animations from changing measured layout.
5. Treat raw HTML according to an explicit trust policy before it reaches Svelte's HTML renderer.

[waku]: https://github.com/egoist/waku
