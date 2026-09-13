# Markdown Pipeline

Vesper compiles Markdown in Rust. The Svelte layer receives rendered HTML and display metadata; it
does not own parsing rules. This keeps Desktop and CLI behavior aligned and prevents consumer-specific
frontend parsers from producing different output for the same Markdown source.

## Compilation contract

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
plain fallback preserves all embed fences as code when enrichment fails. Knowledge removes one leading front matter block before resolving embeds, without interpreting
it as metadata; publication does not apply that stripping.

`render_memo` converts soft line breaks into hard line breaks to preserve the compact writing style
used by my-memos. `compile_knowledge_enriched` assigns stable, de-duplicated heading IDs and produces the table
of contents and excerpt in the same pass boundary as HTML compilation. Consumers continue to own
storage and metadata; Vesper does not retain a second Markdown mirror.

Rust computes reading statistics from prose events, excluding front matter, URLs, code, math, image descriptions and embed configuration; CJK characters use 350/minute and other words use 200/minute. Plain fallback and enriched rendering share these statistics. Embed syntax is validated before external enrichment reads.

The Desktop rich editor compares source and reserialized Markdown through the same Rust parser options used for Knowledge rendering.
Formatting differences such as bullet markers are allowed; changed text, tables, images, raw HTML,
and custom fence languages or bodies prevent switching. Source remains authoritative until an edit.

## Loading and navigation

Knowledge and Newspaper receive a metadata-only index: titles, summaries, tags, dates, edition
classification and content hashes. Listing never reads or compiles article bodies. Opening an article
reads and compiles that document. Visible index entries preload after rendering, with two requests at a time. Pointer intent
(60 ms), keyboard focus and touch also start reads early. Rust shares in-flight reads and caches at most 16 compiled documents for 30 seconds; a changed
content hash forces a new read. Speculative reads are limited to two of six reader slots. Writes and
credential changes invalidate the cache, including results still in flight. A failed preload does
not prevent clicking to retry. These policies apply to Vesper; they are not a static web build.

## Article lists

An explicit `embed:article` fence accepts 1–50 URL-only lines, optionally prefixed with Markdown list bullets. Each entry renders a compact card containing the resolved article title and description. URLs identify destinations and never become card labels. Ordinary Markdown links outside these fences remain ordinary links. Order and repeated entries are retained; metadata reads are deduplicated and bounded.

````markdown
```embed:article
align: narrow
https://knowledge.you-find.me/articles/first-article
https://knowledge.you-find.me/articles/second-article
```
````

The Knowledge consumer resolves Knowledge URLs against the authenticated, paginated summary index.
The index includes both ordinary articles and all daily pages, including historical editions.
It decodes the URL path segment once and matches the web slug to the record’s real ID and supplies title, description, and the desktop
`/articles/<id>` destination without reading or compiling target bodies. Clicking reads the selected
article through the ID-based detail endpoint. The desktop reader opens these cards inside the application, only when resolved from the article index. Stale requests cannot replace a later selection; editing blocks navigation. The web adapter authorizes the same URL against D1 and renders its canonical web route. Missing or unauthorized web targets remain non-clickable.

`embed:article` resolves only articles present in the host's article index. It never fetches other websites or falls back to Open Graph. Missing, external, or unauthorized targets render a non-clickable unavailable card without removing valid siblings. Single-entry `id`/`url` fields and title/description overrides remain supported, but overrides cannot make an unindexed target clickable. Static publication must supply an article index to render navigable article cards. URL navigation uses the same summary-to-ID resolution and does not require a slug-based detail API.

Existing article visibility is a draft property alongside body metadata. Switching it does not persist or navigate; Save sends content and visibility in one request, Cancel discards it, and failures retain the draft. Creation remains public.

Article lists and single-entry cards accept `align: left`, `right`, `wide` (default), or `narrow`. Left and right align a card or the entire list to that side with a maximum width of 32rem; narrow uses the same maximum width and centers it. All fit within the available container. Link, media, GitHub, stock, and SVG embeds accept the same alignment values. Alignment does not change metadata resolution or navigation.

## Audio and video

Use `embed:media` between paragraphs. `type` and `src` are required; `title` supplies the player's
accessible name, `caption` adds visible text, and `align` accepts `left`, `right`, `wide` (default), or `narrow`.
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
