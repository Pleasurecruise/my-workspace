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

Telemetry metrics and the lower Dashboard region use responsive CSS grids. The calendar Todo card
occupies the narrower lower-left column and shows a complete Monday-first month above the selected
date's list. One consolidated panel occupies the wider lower-right column: subscription quotas share
its upper row and monetary/account balances share its lower row. Narrow screens stack the Todo list
and usage panel; provider cells then collapse vertically when needed.

NAS CPU, memory, and network cards pair the latest numeric value with a compact in-session SVG trend
line; storage uses a used/free capacity bar. Weather
cards use a three-column comparison for Shanghai, Ningbo, and Nottingham, with 24-hour local clocks
and six hourly forecast cells; narrow screens stack these cards.

Loading state must preserve already settled information. Initial placeholders belong inside the
affected card; background polling must not replace the entire Dashboard with a loading surface.

## Memo interaction

Memo composer and inline-editor focus belongs to the containing surface: an accent border and subtle
semantic accent halo replace a second textarea ring. Search retains its own accent focus border. Only
one memo can be edited at a time. Choosing another memo first saves the changed draft; an unchanged
draft closes without a request, and a failed save keeps the original editor and draft intact.
The complete tag index sits above search as a horizontally scrollable strip with counts. In the
unfiltered feed, pinned entries live in a collapsed section ahead of the timeline; filtering exposes
matching pinned entries directly. After pinning changes the feed position, the affected entry is
centered in the viewport. Bare web addresses in memo bodies render as links. External links open in
the system browser; a `memos.you-find.me/memo/{id}` link remains in the application, loads successive
feed pages until the target is present, expands the pinned section when needed, then smoothly centers
and briefly highlights the card.
The lower-right Archive and Favorites actions switch the feed between active, archived, and favorite
memos. Selecting the current filter again returns to the active feed. Each view loads its matching
server-side projection, archived entries can be restored from the archive view, and the archive view
omits the memo composer.

## Knowledge interaction

Knowledge article navigation and editing actions live in a quiet side rail on wide layouts and move
as one floating group to the lower-right edge on narrow layouts; they do not compete with the article
heading.

## Moment interaction

Moment keeps the masonry gallery as its default surface. A filter panel owns text search, the complete
tag index, and ascending or descending date order. Selecting a photo opens an application-modal
viewer with keyboard navigation, metadata editing, public-link copying, and confirmed deletion. The
viewer owns scroll focus while open. The inline upload panel pairs the selected image with title,
description, parsed tags, date, and coordinates; validation and upload errors stay in that panel
without replacing settled gallery data.

## Window and theme

The macOS window retains its complete native title bar, including its title, traffic-light controls,
and system-owned drag behavior. Theme changes update the WebView and native appearance directly,
without a page-level transition. Controls do not add independent color transitions that would drift
from the theme change.

The application shell owns one persistent lower-right back-to-top action. Its single progress ring
follows the main canvas scroll position across scrollable views; page-specific actions remain
separate.

The sidebar omits a separate brand header and keeps its navigation visually primary, including
Settings as a full navigation tab. An editable local profile badge anchors the left side of the footer
beside its three controls; selecting it exposes a compact name and avatar editor above the footer. The
square-cropped avatar and display name persist locally and do not imply an application session. On
desktop, the sidebar's right edge supports pointer dragging and keyboard resizing within bounded
widths, and remembers the chosen width. The footer separates destinations from immediate actions
without a visual divider: Inbox remains a destination, while App Lock and theme are immediate actions.
When no password exists, the lock control routes to Settings; otherwise it immediately makes the
complete application shell inert and shows one opaque unlock surface. The unlock form receives focus,
reports an incorrect password in place, and reveals no underlying content. Settings owns password
creation, replacement, and removal, and describes App Lock as a privacy screen rather than encryption.

## Newspaper and Inbox

Newspaper remains in primary navigation and presents the latest Programmer Daily and Personal Daily
articles without an archive. Page-edge arrows switch streams like an album, turning the complete
reading surface instead of exposing a separate tab control; reduced-motion preferences remove the
animation. Its reading surface is distinct from Knowledge: a warm paper field, compact edition line,
editorial masthead, serif hierarchy, restrained ink accent, and dense long-form rhythm inspired by
Kami while still using Vesper's semantic tokens.

The Inbox control in the sidebar footer opens a dedicated empty state independent of content views.
Knowledge articles and Newspaper editions never appear as notifications. Newspaper editions are
reserved for Newspaper and do not appear in the regular Knowledge index.

The operating-system notification adapter is registered but remains dormant. Inbox does not
request notification permission, schedule delivery, or send notifications.

## Accessibility

- Interactive elements use native buttons and inputs.
- Progress indicators expose `role="progressbar"`, a provider-specific accessible label, and numeric
  bounds.
- Errors that require immediate attention use an alert role.
- Icon-only controls require an accessible label.
- Muted text and status colors must remain readable in both themes.
- Motion should use the shared duration tokens and remain limited to meaningful feedback.
