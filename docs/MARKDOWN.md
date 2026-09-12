# Markdown Pipeline

Vesper compiles Markdown in Rust. The Svelte layer receives rendered HTML and display metadata; it
does not own parsing rules. This keeps Desktop and CLI behavior aligned and prevents consumer-specific
frontend parsers from producing different output for the same Markdown source.

## Current pipeline

```text
API or local Markdown
  -> cms-core::markdown
  -> pulldown-cmark events
  -> md-dialect for custom embed:* fences
  -> HTML (plus table of contents and excerpt for Knowledge)
  -> typed Tauri response
  -> Svelte presentation
```

`cms-core::markdown` owns document parsing, HTML assembly, raw-HTML/link policy, and article
metadata. Its article module implements publication and Knowledge compilation. `md-dialect` owns
only custom `embed:*` syntax: field validation, provider snapshot resolution, SVG sanitization,
rendered embeds, and their styles. It returns `None` for ordinary code languages so the caller can
handle them. Consumers access compilation through `cms-core::markdown`.

The existing output profiles remain distinct: publication highlights code with Syntect and renders
Mermaid to SVG; Knowledge preserves ordinary code and Mermaid fences as code, and adds heading IDs,
a table of contents, and an excerpt. Both enriched article paths render custom embeds. Knowledge's
plain fallback preserves all embed fences as code when enrichment fails. Knowledge strips a leading
front matter block without interpreting it as metadata; publication does not apply that stripping.

`render_memo` converts soft line breaks into hard line breaks to preserve the compact writing style
used by my-memos. `compile_knowledge_enriched` assigns stable, de-duplicated heading IDs and produces the table
of contents and excerpt in the same pass boundary as HTML compilation. Consumers continue to own
storage and metadata; Vesper does not retain a second Markdown mirror.

The Desktop rich editor compares source and reserialized Markdown through the same Rust parser options used for Knowledge rendering.
Formatting differences such as bullet markers are allowed; changed text, tables, images, raw HTML,
and custom fence languages or bodies prevent switching. Source remains authoritative until an edit.

## Audio and video

Use `embed:media` between paragraphs. `type` and `src` are required; `title` supplies the player's
accessible name, `caption` adds visible text, and `align` accepts `left`, `right`, or `wide` (default).
Video accepts an optional `poster`; omit it to preview the video’s opening frame automatically. Field values are plain text, with optional surrounding quotes.

````markdown
A recording from the session:

```embed:media
type: audio
src: ./media/interview.mp3
title: Interview recording
caption: The full conversation.
```

A demonstration from a remote source:

```embed:media
type: video
src: https://cdn.example.com/demo.mp4
poster: ./media/demo-cover.jpg
title: Product demonstration
caption: A short walkthrough.
align: wide
```

Continue the article here.
````

For static publication, local paths are relative to the Markdown file: `content/posts/story.md`
can reference `../media/interview.mp3` in `content/media/`. Files must exist inside `content/` and
remain regular copied assets; missing files, outside-root paths, symlinks, and Markdown references
fail the build. Spaces and non-ASCII names are URL-encoded. Use `%23` or `%3F` for literal `#` or `?`
in filenames. Absolute filesystem paths, `~/`, `file:` URLs, and protocol-relative URLs are rejected.
The generated HTML retains relative URLs, so the serving application must preserve the document's
published directory when resolving them, including when rendering `content.json`.

HTTP(S) sources must be playable resources; GitHub file links resolve to raw URLs. Resources are
neither downloaded nor checked for availability during compilation. Knowledge and Newspaper can
render remote media with the same syntax; local assets belong to the `content/` publication workflow,
not the desktop application's filesystem. No upload or local file access is triggered by rendering.

Desktop readers add seeking, volume, errors and video fullscreen; leaving stops playback.
Static output retains native controls: click play to start, with no autoplay
and inline video. A video without `poster` uses `preload="metadata"` and an opening-time fragment
(`#t=0.001`) to request a frame preview; an authored URL fragment is preserved. No image is extracted,
uploaded, or stored. Explicit posters and audio retain `preload="none"`. Browser loading preferences
can defer a preview, and an unavailable or unsupported source cannot provide a frame; controls remain
available. Remote poster images may load with the article. Playback formats and codecs
must be supported by the reader's browser. Published files receive extension-based MIME types
(unknown extensions use `application/octet-stream`), and R2 uploads stream from disk. There is no
transcoding. Keep subtitles or transcripts alongside the media in the article when needed.

Invalid types, duplicate or unsupported fields, unsafe URL schemes, and credential-bearing URLs
fail dialect compilation. Titles and captions are escaped. Knowledge's existing plain fallback
keeps an invalid or unavailable embed visible as source code.

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
