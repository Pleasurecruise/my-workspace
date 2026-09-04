# Design System

Vesper uses one semantic design system across light and dark modes. Application views and reusable
components consume semantic tokens rather than physical colors.

## Token architecture

```text
palette.css  -> physical light/dark values
tokens.css   -> semantic --color-*, --font-*, --radius-*, --shadow-*, --duration-* roles
theme.css    -> Tailwind utility mapping
```

Rules:

- `--palette-*` variables may appear only in `tokens.css`.
- Components and application styles use semantic tokens only.
- Add a new physical value to `palette.css`, assign it a semantic role in `tokens.css`, then expose
  it through `theme.css` only when a utility mapping is needed.
- Do not place raw hexadecimal, RGB, HSL, or framework palette colors in application components.
- Dynamic numeric values such as progress width may use an inline style; static visual decisions
  belong in classes and tokens.

## Component ownership

`packages/ui` owns primitives that are reusable across applications: buttons, inputs, labels,
textareas, cards, alerts, badges, and design tokens. `apps/desktop/src/lib/components` owns composed
views and desktop-specific interaction.

Do not move a component into `packages/ui` solely to shorten an import. Promote it only when its API
is stable and it has more than one plausible application consumer.

## Dashboard layout

Dashboard uses a user-configurable fixed desktop widget canvas. Edit mode uses a four-way move
pointer to drag the card itself, with no dedicated handle or card-level component menu. Each card has
one small upper-right delete action. Cross-row movement inserts at row boundaries, preserving each
row instead of splitting it around a full-width card. The Add Widget action opens a searchable library with a preview;
weather accepts a user-entered place and stock accepts a ticker symbol. The canvas retains twelve
tracks and scrolls horizontally when space is limited, so a breakpoint never rewrites the saved
order.

Calendar and Todo are separate widgets that share the selected date. The widget library uses a
category rail without a search field. One System Status category contains UGREEN CPU, UGREEN Memory,
UGREEN Storage, and UGREEN Network alongside Device CPU, Device Memory, Device Storage, and Device
Network. Quota and Balance each list one independent widget per provider rather than a provider
table. Provider-card content is vertically centered in its card.
Todo titles are buttons that switch the fixed-size card from its list to a detail view. The detail
view always shows the selected date and status, adds calendar timing, location, and description for
ICS items, keeps long content in an internal scroll area, and uses a back action to restore the list
without resizing the dashboard layout.

NAS CPU, memory, and network cards pair the latest numeric value with a compact in-session SVG trend
line; NAS storage uses a used/free capacity bar. Device Storage shows startup-disk used, total and
free capacity in decimal GB, followed by the complete category breakdown. A separate rescan control
and timestamp describe the estimates; partial scans remain labeled and do not replace live capacity.
Each weather card shows one configured city with a 24-hour local clock and six hourly forecast cells. Stock cards show a configured ticker's current
price, daily change, and recent trend. The optional exchange card uses the same four-track footprint
to compare USD, GBP, and EUR against CNY, emphasizing USD/CNY while keeping the provider date visible.
Service-status cards pair overall health with the names and states of affected services. The list
scrolls within the card when needed; a healthy card shows “All services operational.” The health
bar exposes numeric bounds to assistive technology. Users select a service from the widget catalog.

The GitHub card places recent activity and unread notifications beside the contribution calendar.
The calendar opens at the most recent dates; notifications show their reason and repository, with
review requests explicitly labeled. Long notification lists scroll within the card, and Open inbox
leads to GitHub. Notification failures remain local to that section.

Loading state must preserve already settled information. Initial placeholders belong inside the
affected card; background polling must not replace the entire Dashboard with a loading surface.

## Memo interaction

Memo composer and inline-editor focus belongs to the containing surface: an accent border and subtle
semantic accent halo replace a second textarea ring. Search retains its own accent focus border. Only
one memo can be edited at a time. Choosing another memo first saves the changed draft; an unchanged
draft closes without a request, and a failed save keeps the original editor and draft intact.
Tag completion supports Up/Down selection, Enter or Tab insertion, and Escape dismissal without
interfering with input-method composition.

Automatic pagination that fills a short consumer view remains visually quiet. The shared loading-more
status appears only when reaching the end through user scrolling, while settled content stays visible.
The complete tag index sits above search as a horizontally scrollable strip with counts. In the
unfiltered feed, pinned entries live in a collapsed section ahead of the timeline; filtering exposes
matching pinned entries directly. After pinning changes the feed position, the affected entry is
centered in the viewport. Bare web addresses in memo bodies render as links. External links open in
the system browser; a `memos.you-find.me/memo/{id}` link remains in the application, loads successive
feed pages until the target is present, expands the pinned section when needed, then smoothly centers
and briefly highlights the card.

The lower-right Archive and Favorites actions switch the feed between active, archived, and favorite
memos. Selecting the current filter again returns to the active feed. Each view loads its matching
server-side projection. Archive and Favorites follow the consumer's compact month-grouped reading
layout and do not inherit the active feed's tag index, search, or sort controls. Archived entries can
be restored or permanently deleted from their row; Favorites owns the X/Twitter import field. Memo
creation, editing, favorite, archive, sharing, and deletion actions report completion through
non-blocking application toasts while request errors remain visible in the active surface.

Only public Memo cards expose outbound publication. Their Telegram paper-plane and X icons sit
immediately after the `public` label in the card header; private cards render no publication control.

## Knowledge interaction

Knowledge article navigation and editing actions live in a quiet side rail on wide layouts and move
as one floating group to the lower-right edge on narrow layouts; they do not compete with the article
heading.
Article creation and editing use a rich-text toolbar for headings, emphasis, lists, quotes, code, and
links. A Markdown source mode remains available, and the editor selects it instead of silently
rewriting content that the rich-text schema cannot represent exactly. The reader and remote storage
keep Markdown as their one source format.

Compiled GitHub and stock embeds use an Innei-inspired editorial card treatment: warm paper, fine
outlines, light shadows, compact line icons, and one muted accent. The compiler emits the styles.
Architecture canvases use transparent authored SVG with Claude-style rounded nodes, curved
connectors, compact type hierarchy, and semantic muted color groups. Storyboard canvases use
irregular paths, offset double strokes, round-ended scribbles, and a handwritten CJK-capable font
stack. They remain transparent and frame-free so the consumer theme stays visible.

## Moment interaction

Moment opens on a masonry gallery. Upload is a quiet action beside the Gallery title and enters a
focused upload view; Filter remains a separate header action. The filter panel owns the complete tag
index, Any/All matching, and date order, with active tags summarized as removable chips.

Selecting a photo opens an application-modal viewer with keyboard navigation, contained scroll
focus, and a toolbar for sharing, editing, deleting, and closing. The preview remains visible until
the original is decoded and ready to replace it. Upload combines preview, title, description, tags,
date, and coordinates; errors stay within the active surface while settled gallery content remains
visible.

## Music interaction

Music follows Moment in primary navigation. The upper-right segmented switch replaces search and
selects Spotify Liked Songs or QQ Music Daily 30. Each source retains its last settled collection
while mounted; switching resets the dedicated player surface to the selected source's list. Music
does not expose browse, external search, or additional playlist navigation. Selecting a row starts
local playback and replaces the library with a dedicated player. Returning through primary Music
navigation restores the current source's library.

The player is one centered column: rotating vinyl, one rolling lyric subtitle, track metadata,
transport controls, and a seekable timeline with elapsed and total time. Synced lyrics follow their
timestamps; plain lyrics advance proportionally. Previous and Next use the loaded library order,
while one mode control cycles sequential, repeat-one, and shuffle for Rust-owned track-boundary
advancement. There is no separate lyric card or scrollable lyric pane. Reduced-motion preferences
stop vinyl and subtitle animation without hiding playback state.

## Window and theme

The macOS window retains its complete native title bar, including its title, traffic-light controls,
and system-owned drag behavior. Theme changes update the WebView and native appearance directly,
without a page-level transition. Controls do not add independent color transitions that would drift
from the theme change.

The application shell owns one persistent lower-right back-to-top action. Its single progress ring
follows the main canvas scroll position across scrollable views; page-specific actions remain
separate.

The startup update check stays out of navigation flows. When a signed update is available, a modal
presents the new version, release notes, and installed version before downloading. Installation
requires an explicit user action, reports progress without hiding errors, and restarts only after
signature verification and installation complete successfully. Check failures use transient,
dismissible feedback rather than a persistent application state. The native application menu offers
an explicit Check for Updates action; a manual check reports when the installed version is current.

The sidebar omits a separate brand header and keeps its navigation visually primary, including
Settings as a full navigation tab. An editable local profile badge anchors the left side of the footer
beside its three controls; selecting it exposes a compact floating name and avatar editor above the
footer. The editor receives focus when opened and collapses when focus leaves the badge and popover. The
square-cropped avatar and display name persist locally and do not imply an application session. On
desktop, the sidebar's right edge supports pointer dragging and keyboard resizing within bounded
widths, and remembers the chosen width. The footer separates destinations from immediate actions
without a visual divider: Inbox remains a destination and shows a small status dot while unread
notifications exist, while App Lock and theme are immediate actions.
When no password exists, the lock control routes to Settings; otherwise it immediately makes the
complete application shell inert and shows one opaque unlock surface. The unlock form receives focus,
reports an incorrect password in place, and reveals no underlying content. Settings owns password
creation, replacement, and removal, and describes App Lock as a privacy screen rather than encryption.

## Newspaper and Inbox

Newspaper presents the latest Programmer Daily and Personal Daily articles without an archive.
Page-edge arrows turn the reading surface like an album and start the selected edition at the top;
background refreshes preserve the reading position. Reduced-motion preferences disable the turn
animation. The Kami-inspired reading surface uses warm paper, a compact edition line, an editorial
masthead, serif hierarchy, restrained ink, and dense long-form rhythm with Vesper's semantic tokens.

The Inbox control in the sidebar footer opens a dedicated empty state independent of content views.
Knowledge articles and Newspaper editions never appear as notifications. Newspaper editions are
reserved for Newspaper and do not appear in the regular Knowledge index.

New live ntfy messages can use the operating-system notification adapter while replayed messages
only populate Inbox. Settings presents one concealed ntfy token field. The endpoint and topic are
fixed application policy, and producer routes and secrets remain outside Vesper.

## Accessibility

- Interactive elements use native buttons and inputs.
- Progress indicators expose `role="progressbar"`, a provider-specific accessible label, and numeric
  bounds.
- Errors that require immediate attention use an alert role.
- Icon-only controls require an accessible label.
- Muted text and status colors must remain readable in both themes.
- Motion should use the shared duration tokens and remain limited to meaningful feedback.
