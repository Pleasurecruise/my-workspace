# Markdown Pipeline

Rust owns Markdown compilation; Svelte receives HTML and display metadata. Consumers call
`cms-core::markdown`, which parses documents and assembles HTML. `md-dialect` validates and renders
custom `embed:*` fences, resolves provider snapshots, sanitizes SVG, and supplies embed styles.

## Compilation

| Profile            | Output                                                                             |
| ------------------ | ---------------------------------------------------------------------------------- |
| Publication        | Syntect-highlighted code, Mermaid SVG, enriched embeds                             |
| Knowledge          | Ordinary code/Mermaid fences, enriched embeds, stable heading IDs, TOC and excerpt |
| Knowledge fallback | Embeds remain visible as source code                                               |
| Memo               | Soft line breaks become hard breaks                                                |

Knowledge strips one leading frontmatter block without using it as metadata. Publication does not.
Embed validation precedes provider reads. Rust counts prose at 350 CJK characters or 200 other words
per minute, excluding frontmatter, URLs, code, math, image descriptions and embed configuration.
Consumers own storage and metadata; Vesper keeps no second Markdown mirror.

Rich/source switching compares Markdown semantics: formatting differences are allowed, content
changes are not. Source stays authoritative until edited. Loading, cache invalidation and navigation
belong to [Architecture](ARCHITECTURE.md#desktop-boundary); draft visibility and Save behavior belong
to [Design](DESIGN.md).

## Shared embed rules

Fields use `name: value`, optionally quoted. Unknown, duplicate or invalid fields fail compilation;
text is escaped. Sources require credential-free HTTP(S), except media also accepts document-relative
assets. Ordinary links and code languages remain unchanged.

All embeds accept `align`: `wide` fills the container; `left`/`right` cap width at 32rem and align to
that edge; `narrow` centers the same width. Alignment never changes authorization or navigation.

## Provider cards

| Fence          | Required field             | Resolved content    |
| -------------- | -------------------------- | ------------------- |
| `embed:github` | `repo: owner/name`         | Repository metadata |
| `embed:stock`  | `code: AAPL`               | Stock price series  |
| `embed:link`   | `url: https://example.com` | Website preview     |

The shared `quotes` providers resolve these cards during compilation. Author text fields as plain
text; provider data never rewrites the stored Markdown. An enrichment failure invokes the host's
fallback policy described above.

## Article cards

`embed:article` accepts 1–50 URL lines, optionally prefixed with list bullets. Order and duplicates
are preserved; metadata reads are deduplicated. Cards display resolved titles and descriptions.

````markdown
```embed:article
align: narrow
https://knowledge.you-find.me/articles/11111111-1111-4111-8111-111111111111
https://knowledge.you-find.me/articles/22222222-2222-4222-8222-222222222222
```
````

Only the host's authorized article index can resolve cards; there are no external previews or
Open Graph fallbacks. Missing, external or unauthorized entries remain non-clickable without hiding
valid siblings. Vesper also accepts single-entry `id`/`url` fields and title/description overrides;
overrides cannot authorize a target. Static publication must supply an index.

Knowledge resolves UUIDs through the authenticated summary index, including historical daily
editions, then opens details by ID. Links retain chapter
fragments, which the destination reader decodes after mounting. Card rendering never reads target
bodies. The web adapter separately authorizes against D1. Editing blocks navigation; stale responses
cannot replace a later selection. Shared Knowledge submissions use URL lines or a single `url`
field; `id`, title and description overrides are Vesper-only extensions.

## Audio and video

`embed:media` requires `type: audio|video` and `src`. Optional fields are `title` (accessible name),
`caption`, `align`, and video-only `poster`.

````markdown
```embed:media
type: video
src: https://cdn.example.com/demo.mp4
title: Product demonstration
caption: A short walkthrough.
```
````

Playback is manual and video stays inline. Without a poster, video requests an opening-frame preview
using `preload="metadata"` and `#t=0.001`, preserving authored fragments. Audio and explicit posters
use `preload="none"`. Preview availability and codecs depend on the browser. Desktop adds seeking,
volume, errors and fullscreen; leaving stops playback. No transcoding or preview image is stored.

Local assets resolve relative to the Markdown file and must be regular files inside `content/`.
Missing files, symlinks, outside-root paths and Markdown targets fail the build. Absolute filesystem
paths, `~/`, `file:` and protocol-relative URLs are rejected. Spaces/non-ASCII names are encoded;
use `%23`/`%3F` for literal filename characters. Serving applications must preserve published path
resolution, including for `content.json`. Uploads stream with extension-based MIME types, defaulting
to `application/octet-stream`.

Remote media is not downloaded or availability-checked during compilation; GitHub file pages become
raw URLs. Local paths belong to static publication, not desktop filesystem access or an upload flow.

## Quotes and Git diffs

Both use metadata, an exact `---` separator, then plain text. Quote requires `author`; `title` and
HTTP(S) `url` are optional. It preserves whitespace without parsing nested Markdown or fetching
source metadata.

````markdown
```embed:quote
author: Project notes
url: https://example.com/source
---
Keep the knowledge and its context.
```

```embed:diff
title: Publication default
---
--- a/config.ts
+++ b/config.ts
@@ -1 +1 @@
-const visibility = "private";
+const visibility = "public";
```
````

Diff requires `title` and a complete unified text patch with matching hunk counts. Multiple files,
hunks, new/deleted text files and no-final-newline markers are supported; binary/mode-only patches
and extra blank patch lines are rejected. Git never runs. Colored rows retain plus/minus markers
and keyboard scrolling. Ordinary `diff` fences remain code.

Both profiles compile these blocks without provider reads. Knowledge submissions retain semantic
fences for the web compiler. [Shared fixtures](../crates/md-dialect/tests/fixtures/document-embeds.json)
specify the contract with my-knowledge.

## Annotations

`embed:annotation` uses `mark`, `note`, optional `color` and HTTP(S) `url`, followed by an exact
`---` and a plain-text body. The mark must occur exactly once, including overlapping occurrences.
Colors are blue (default), red, green, amber, and purple. The compiler preserves the sentence,
highlights the mark, and places the note below it with a matching border; it does not inject scripts
or reproduce the web reader's measured arrow. Text remains readable without JavaScript.

## Engineering diagrams and storyboards

Architecture supports `flowchart LR`/`graph LR`, one two-endpoint edge per line, and optional
`[labels]`. Structured engineering diagrams group nodes by dependency; siblings share a column.
Labels wrap at words or grapheme boundaries; node heights and edge anchors grow with their lines.
Containers at most 640px wide use a vertical layout with at most two nodes per row. Cycles and
skipped layers route around nodes. Generated diagrams grow in height without the authored SVG
height cap. The query measures the diagram container after alignment, so left/right/narrow diagrams
also select the compact layout when the article itself is wide.

Storyboard supports one title and two to six `step: heading | description` fields. Both kinds also
accept authored SVG with a viewBox, title and description. SVG is sanitized without rearranging its
geometry; authored canvases retain a 42rem display height cap. Keep canvases transparent and frame-free.
Architecture SVG uses rounded `.node` groups, `.arr`/`.leader` paths, `.th`/`.t`/`.ts` text and
`.c-purple`, `.c-teal`, `.c-coral`, `.c-blue`, `.c-green`, `.c-amber`, `.c-red`, `.c-gray` groups.
Storyboard uses irregular paths, `.scribble`, `.arrow`/`.arrow-shadow`, `.sketch-shadow`, `.hand`
text and `.fill-blue`, `.fill-violet`, `.fill-green`, `.fill-orange` groups.

## Showing source

Use longer backtick fences or tildes to show dialect source. For compatibility with Knowledge,
a bare triple-backtick wrapper immediately around one embed, with adjacent inner/outer closers,
is normalized to a Markdown code example. Provider collection uses the same normalization, so
examples never trigger enrichment and following live embeds still compile.

## Image shortcodes

Use named green-cat shortcodes such as `:suzume_思考:` and `:suzume_期待:`,
`:suzume5_01:`–`:suzume5_30:`, or `:baishengnv_01:`–`:baishengnv_117:` in prose.
Matching is case-sensitive. Unknown shortcodes remain literal. Code, links (including labels),
image alt text, math and generated embeds are not expanded. The compiler never downloads images.

`crates/md-dialect/src/emoji-packs.json` owns the catalog; keep it identical to my-knowledge's
`packages/content/src/emoji-packs.json`. Each pack has an ASCII alphanumeric `key`, a `name`,
`display` (`emoji` or `sticker`), and `items` with `name` and HTTPS image `value`.
Names exclude whitespace and colons; duplicate shortcodes and credential-bearing URLs are invalid.
Never reuse published names for different artwork.

The preferred `suzume` pack references 16 named 300×300 green-cat images from
[Suzume’s public collection](https://szm.de5.net/posts/suzume5/), excluding its contact sheet.
Existing SuzumeS5 and 白圣女 names retain their Fullyst and Stickers.wiki URLs.
The low-resolution Combot packs (`daimao2`, `denghuoju8`) were removed at the owner's request;
update their article shortcodes before deploying the removal. These are remote references, not
bundled artwork or a license grant. Rights and availability remain with authors/providers.
