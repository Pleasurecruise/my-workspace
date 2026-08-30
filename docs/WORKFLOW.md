# Local-to-Consumer Workflow

Vesper has two delivery paths. Static publication uploads a complete local build to R2. Consumer
operations update one deployed application through its authenticated API, with direct R2 access only
where binary transfer is intentionally owned by the local client.

## Infrastructure map

```mermaid
flowchart TD
    Source[Local Markdown and assets] --> Build[build.rs]
    Build --> Stage[Temporary build directory]
    Stage --> Publish[publish.rs]
    Publish -->|S3-compatible SDK| BlogR2[(R2 blog/)]
    BlogR2 --> StaticConsumer[Static consumer]

    Vesper[Vesper desktop or CLI] --> MemoAPI[my-memos REST API]
    MemoAPI --> MemoD1[(D1 metadata)]
    MemoAPI --> MemoKV[(KV cache)]
    MemoAPI --> MemoR2[(R2 memo bodies)]

    Vesper --> KnowledgeAPI[my-knowledge REST API]
    KnowledgeAPI --> KnowledgeD1[(D1 metadata and authorization)]
    KnowledgeAPI --> KnowledgeKV[(KV projection)]
    KnowledgeAPI --> KnowledgeR2[(R2 article bodies)]

    Vesper -->|image bytes through R2 SDK| MomentR2[(R2 images)]
    Vesper -->|image keys and metadata| MomentAPI[my-moment REST API]
    MomentAPI --> MomentD1[(D1 photo metadata)]
    MomentD1 --> MomentConsumer[my-moment]
    MomentR2 --> MomentConsumer
```

## Static publication

`vesper build` delegates to `build.rs`. The builder walks `content/`, renders Markdown through
`markdown.rs`, highlights fenced code with Syntect, renders `mermaid` fences to SVG with the
pure-Rust `mermaid-svg` renderer, copies regular assets, rejects symbolic links and output
collisions, and writes a `content.json` index into an operating-system temporary directory. Code
styles and Mermaid SVG styles are embedded in the HTML artifact, so consumers do not run either
renderer. Invalid or unsupported Mermaid input fails the build before an upload plan exists.

`vesper publish` builds the same directory and reports the planned object count. It does not mutate
remote state. `vesper publish --live` passes the staged files to `publish.rs`, which uploads them
through `r2.rs` below the `blog/` prefix. The temporary directory is removed when its Rust guard is
dropped. Publication is additive and does not delete destination-only objects.

## Memos

Memo metadata operations use the my-memos REST API. Create, update, and delete requests must pass
through the Worker so R2 bodies, D1 metadata, and KV invalidation remain one server-coordinated
transaction. List and search return the D1 body mirror with each `r2Key`; Vesper renders that body
locally without a second R2 read.

CLI surface:

```text
vesper memo tags
vesper memo list [limit]
vesper memo page <json>
vesper memo search <query>
vesper memo create <markdown>
vesper memo import-x <url> [public|private]
vesper memo update <id> <markdown>
vesper memo patch <id> <json>
vesper memo visibility <id> <public|private>
vesper memo pin <id>
vesper memo unpin <id>
vesper memo favorite <id>
vesper memo unfavorite <id>
vesper memo archive <id>
vesper memo restore <id>
vesper memo delete <id>
```

`memo page` accepts `cursor`, `limit`, `search`, `tags`, `sortByUpdated`, `archivedOnly`, and
`favoritesOnly`, matching desktop feed reads. The two final filters are mutually exclusive. `memo
patch` accepts the same optional `content`, `visibility`, `tags`, `pinned`, `favorite`, and
`archived` fields as the desktop command contract and rejects an empty object.
`memo import-x` uses the same Rust FxTwitter import and favorite-creation flow as the desktop. It
creates a private favorite by default; pass `public` explicitly to publish it.

## Knowledge

Knowledge always enters through the authenticated my-knowledge REST API. The Worker performs D1
authorization before resolving KV or R2 data. Updates and deletion include the current
`expectedHash`; a stale local copy fails instead of overwriting a newer article.
The desktop Knowledge editor creates drafts and sends edits through the same API, preserving the
loaded content hash for conflict detection.

Complex create and update payloads are passed as one quoted JSON argument. This keeps multilingual
documents and optional fields aligned with the API contract rather than inventing a second CLI
schema.

```text
vesper knowledge list [cursor]
vesper knowledge get <id>
vesper knowledge create <json>
vesper knowledge update-draft <id> <json>
vesper knowledge update-documents <id> <json>
vesper knowledge visibility <id> <json>
vesper knowledge delete <id> <expected-hash>
```

## Moment

Moment separates local image preparation, binary transfer, and metadata coordination. The shared
Rust path prepares one source image, uploads its normalized original and thumbnail to R2, then
registers their exact object keys with the REST API. The consumer resolves metadata from D1 and reads
the image objects from R2.

```text
vesper moment upload-photo <json> <source-image>
vesper moment upload <r2-key> <local-path>
vesper moment create '<json-with-r2Key-and-thumbnailR2Key>'
```

`moment upload-photo` is the coordinated path shared with the desktop. Its JSON uses the `Upload`
contract and the source may be PNG, JPEG, WebP, AVIF, or HEIC up to 20 MB. Rust applies camera
orientation, uses available EXIF time and coordinates when the JSON omits them, derives the normalized
PNG, JPEG thumbnail, and ThumbHash, then uploads both objects and registers metadata. Objects written
by the operation are removed if a later step fails. The lower-level `upload` and `create` commands
remain available for explicit recovery workflows.

If metadata registration fails, the uploaded objects are orphans. Inspect the error, retry the same
metadata request, or explicitly remove the orphan with `vesper moment remove-object <r2-key>`. Do not
remove an object referenced by an existing photo record. `moment delete <id>` is the normal metadata
deletion path; raw object removal is an explicit maintenance operation.

The remaining Moment commands cover tags, listing, search, metadata updates, downloads, and deletion:

```text
vesper moment tags
vesper moment list [cursor]
vesper moment search <query>
vesper moment update <id> <json>
vesper moment download <r2-key> <local-path>
vesper moment delete <id>
```

The desktop sends the original image and user-entered metadata into the same Rust workflow. Its
viewer sends title, description, and tag edits to the authenticated Moment update endpoint and sends
confirmed deletion through the Moment delete endpoint, which remains responsible for coordinating
metadata and stored-image removal.

## Todo

Todo operations are local and require no credential. Desktop and CLI share the date-keyed
`todos.json` file in Vesper's operating-system application data directory. The previous
`today-todos.json` file is ignored without fallback or migration. CLI commands infer the current
local date unless prefixed with `todo --date YYYY-MM-DD`; the desktop month calendar can select any
date. At local midnight Rust
advances today's view to the new empty list without deleting previous lists.

```text
vesper todo list
vesper todo get <id>
vesper todo create <text>
vesper todo update <id> <text>
vesper todo complete <id>
vesper todo reopen <id>
vesper todo delete <id>
```

For any action above, use `vesper todo --date <YYYY-MM-DD> <action> [...]` to operate on the same
historical or future day selectable in the desktop.

## Credentials and failure boundaries

`vesper status` reads UGOS and the four shared AI-provider integrations concurrently. Its JSON keeps
each source's success or failure independent and does not expose credentials. Weather and GitHub
remain desktop-local integrations.

Release builds read consumer API keys and R2 credentials from the operating-system credential store.
Debug builds can use the variables listed in `.env.example` to avoid repeated macOS Keychain prompts
from ad-hoc-signed development binaries. Provider failure is isolated: an API metadata failure does
not silently become an R2 success, and an R2 upload does not imply that consumer metadata exists.
