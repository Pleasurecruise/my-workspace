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
textareas, selects, cards, alerts, badges, and design tokens. `apps/desktop/src/lib/components` owns composed
views and desktop-specific interaction.

Do not move a component into `packages/ui` solely to shorten an import. Promote it only when its API
is stable and it has more than one plausible application consumer.

## Page layout and typography

The application shell owns one centered frame in `components/layout/page.css` for every page.
Its maximum width is 84rem including 2rem side padding, leaving up to 80rem for content. Below
768px side padding is 1rem. Pages fill the available content width; there is no route-specific
wide/narrow classification. Automatic inline margins keep the frame centered within the main area
as the sidebar resizes, and headings align with their own page content. Desktop top padding is
2rem; mobile top padding is 1.5rem.

Loading, errors, reading, and editing use the same frame. Newspaper retains its centered 58rem
paper surface as part of its internal composition; page cards and controls retain their own
responsive arrangements.
The shell measures the content slot for the Music list, Knowledge editor fields, and Memos import
controls to stack at ≤640px, including when the sidebar is resized. Settings form rows already
respond to their own content container; footers wrap when needed. Other existing page compositions
retain their own breakpoints. The frame does not establish containment around
viewport-fixed dialogs.
Shared headers use `page-header`, with the H1 first, an optional `page-description` below it, and
wrapping actions aligned to the top. Titles, descriptions and header spacing use the shared rules
without page-specific overrides. The main scroller reserves its scrollbar gutter to avoid horizontal
shifts when content overflows. The page frame alone supplies the top inset; loading, ready, and error headers
start at that same baseline. Paper padding applies only to Newspaper body content and placeholders,
never to its page header. Tag status stays below the header so background reads cannot shift it.

Page and section headings use the shared semantic scale by default. Existing content compositions
own their internal typography: Dashboard cards, Moment's gallery controls and viewer, and Newspaper's
editorial surface opt out through `data-content-typography`. An explicit `page-title` still uses the
shared H1 typography inside these surfaces, as Moment does for its page heading. Shared frame changes must not change
widget spans, gallery grouping, paper treatment, or article typography. H1–H6 defaults are 2rem,
1.125rem, 0.9375rem, 0.875rem, 0.8125rem, and 0.75rem; the page title uses the serif family.

## Dashboard layout

Dashboard uses a configurable twelve-track canvas with 0.5rem gaps and generally 0.75rem card
insets. Its height follows its content within the shared page frame. Edit/Done and Refresh icons
sit beside the page title; Restore Default and Add Widget appear at the trailing edge in edit mode.
Cards expose removal and pin controls while editing, and the card surface acts as the drag handle.
Cross-row dragging inserts at row boundaries. Cards retain their configured spans and order as the
window changes width; compact three-track cards share an 8.5rem minimum height.

Add Widget opens a category rail, widget list, and preview with feature-specific configuration.
Weather accepts a location and stocks accept a ticker. Personal includes Daily Planner and Spending; Devices includes
UGREEN and current-device telemetry. AI Services groups usage and balance provider widgets; Online Services and Games contain their respective integrations. Provider-card content
is vertically centered within its card.

Daily Planner spans all twelve tracks with Calendar on the left, Todo in the middle, and daily
habits on the right inside one border. The sections share 0.75rem insets and 1.75rem heading rows.
Todo and habit titles use 0.7rem regular-weight text and compact icon actions. Fixed-height content
areas keep scrolling inside lists and details; the Dynamic Island stacks the three sections.

Calendar selection controls both the Todo and habit date, and stays available while tasks load. Todo completion uses checkboxes; titles open a detail
view with status, date, description, and available calendar metadata. Back restores the list without
resizing the card. Quick add offers an expandable description, and manual-task details provide an
editor with Save and Cancel. Failed saves preserve the draft; duplicate submissions are disabled.
Imported items direct content changes to the source calendar.

Habits show the selected date explicitly beside their ongoing streak, total days through that date,
and the 28-day history ending on it. Historical dates support check-in and undo. Future dates show
a read-only explanation and disabled check-in actions. Each row has its own check-in/undo icon, keeping the action distinct from Todo's
completion checkbox. Manage opens the habit-name form and individual removal controls. Habit reads
happen automatically and need no separate refresh control.

Spending is a separate full-width card linked to the Calendar date. Its header provides
previous/next-day navigation and Today. The left column holds the day's total, a compact GBP amount
and category form, and a scrolling list with edit/delete actions. Categories use the shared Select in its compact size, matching the amount input’s 2rem height,
font, and padding. Custom category and optional note fields span the next rows; saved notes appear
below each entry’s category. The shared Select keeps its focused option visible during keyboard
navigation and handles empty or disabled option lists. There is no second calendar or refresh control.

The right column shows the monthly total and switchable category donut or daily bars in a fixed
9rem chart region. Chart changes preserve drafts and card height. Read-only bars expose dates and
GBP amounts and highlight the selected day. Empty months show a neutral ring, explicit copy, and
zero daily bars. Narrow surfaces and Dynamic Island stack the columns. Semantic tokens
`--color-chart-1` through `--color-chart-7` provide colors in both themes; labels and amounts keep
charts understandable without color discrimination.

An invalid widget displays its saved configuration as escaped text and collapsible error details
in its original position. Dashboard edit controls can remove it; other cards remain usable. This
diagnostic is not offered in the widget library.

NAS CPU, memory, and network cards pair the latest numeric value with a compact in-session SVG trend
line; NAS storage uses a used/free capacity bar. Device Storage shows startup-disk used, total and
free capacity in decimal GB in a card stretched to the same row height as Device CPU and Stock.
Three-track widgets share a minimum height; storage contents never contribute to the row height. Category details switches to the complete
breakdown in an internal scroll area; Back to capacity restores the overview without resizing the
row. A separate rescan control and timestamp describe the estimates; partial scans remain labeled and do not replace live capacity.
Each weather card shows one configured city with a 24-hour local clock and six hourly forecast cells. Stock cards show a configured ticker's current
price, daily change, and recent trend. The optional exchange card uses the same four-track footprint
to compare USD, GBP, and EUR against CNY, emphasizing USD/CNY while keeping the provider date visible.
Service-status cards pair overall health with the names and states of affected services. The list
scrolls within the card when needed; a healthy card shows “All services operational.” The header
stays at the top, while the status summary, health bar, numeric labels, and incident count form one
naturally spaced group centered vertically in the remaining content area. The health bar exposes
numeric bounds to assistive technology. Users select a service from the widget catalog.

The GitHub card places recent activity and unread notifications beside the contribution calendar.
The calendar opens at the most recent dates; notifications show their reason and repository, with
review requests explicitly labeled. Long notification lists scroll within the card, and Open inbox
leads to GitHub. Notification failures remain local to that section. Arknights and Endfield use
a compact two-column daily summary and smaller archive rings to match the GitHub card’s
visual density. Cards grow naturally without fixed-height scrolling; archive guidance is
available through an expandable disclosure.

Loading state must preserve already settled information. Initial placeholders belong inside the
affected card; background polling must not replace the entire Dashboard with a loading surface.
First-load consumer skeletons keep the shared page header at its settled position: Memos shows a
composer and cards, Moment a responsive photo grid, Knowledge dated article rows, and Newspaper an
article masthead and paragraphs. Music uses track rows below its existing header. Decorative
placeholders stay hidden from assistive technology, the loading region announces its state, and
reduced-motion preferences disable animation.

## Dynamic Island

On macOS, a transparent native window places the island at the top center of the primary screen in
the menu-bar area. The visual reference is [Atoll](https://github.com/Ebullioscopic/Atoll): outward top
shoulders, rounded lower corners, a continuous black surface, and restrained controls. Notched
screens place the collapsed icon and status on opposite sides of the notch; other screens use a
compact pill. The expanded frame is up to 560 logical pixels wide, with room below the notch for a
heading and scrollable widget content. Scoped semantic tokens keep it dark independently of the
main window's theme.

Dashboard edit mode pins a saved placement; Daily Planner is selected by default. Unpinning or
removing the placement disables the island. Hover, click, or keyboard activation expands it. Escape,
Close, or leaving without pointer/focus inside collapses it. Initial display uses AppKit's non-key
ordering operation. Subsequent frame changes preserve the top anchor and honor Reduce Motion.
Losing focus does not reactivate the island. App Lock closes it, and other platforms omit pinning.

The island shares WidgetContent rendering with Dashboard in a separate WebView session. Embedded
content omits its outer card frame. Daily Planner stacks Calendar, Todo, and habits; Todo retains
its circular completion controls, add, delete, and detail actions. Task and habit mutations update
the other surface, and expanded Planner content rereads once a minute. Provider widgets read on
expansion without enabling Dashboard polling. Display changes reposition the window on its next
expansion or collapse.

## Memo interaction

Memo composer and inline-editor focus belongs to the containing surface: an accent border and
subtle semantic halo replace a second textarea ring. Search retains its own focus border. Composer
text and the active edit survive switching pages during the current session. A successful save clears an
unchanged draft; text changed while a request is pending remains editable. Selecting another memo
saves the current edit first and switches only when no unsaved changes remain. Cancel explicitly
discards the active edit. Tag completion supports Up/Down, Enter/Tab, and Escape without interfering
with input-method composition.

Automatic pagination that fills a short consumer view remains visually quiet. The shared loading-more
status appears only when reaching the end through user scrolling, while settled content stays visible.
The complete tag index sits above search as a horizontally scrollable strip with counts. Small edge
arrows appear when tags extend beyond that edge and scroll directly to the start or end. Their
visibility follows scrolling, tag changes, and resizing; reduced motion disables smooth scrolling. Selected
tags have separate removal controls, so filters remain removable when a refreshed index no longer
contains them. Tag loading and retryable errors remain separate from the feed and preserve the last
successful index. In the unfiltered feed, pinned entries live in a collapsed section ahead of the timeline; filtering exposes
matching pinned entries directly. After pinning changes the feed position, the affected entry is
centered in the viewport. Bare web addresses in memo bodies render as links. External links open in
the system browser; a `memos.you-find.me/memo/{id}` link remains in the application, loads successive
feed pages until the target is present, expands the pinned section when needed, then smoothly centers
and briefly highlights the card.

The lower-right Archive and Favorites actions switch the feed between active, archived, and favorite
memos. Selecting the current filter again returns to the active feed. Each view loads its matching
server-side projection. Archive uses the compact month-grouped reading layout without tag, search,
or sort controls; entries can be restored or permanently deleted. Favorites uses the regular Memo
cards with inline editing, deletion, favorite, pin, archive, and sharing controls. It retains tag,
search, and sort controls plus the X/Twitter import field. Memo
creation, editing, favorite, archive, sharing, and deletion actions report completion through
non-blocking application toasts while request errors remain visible in the active surface.

Only public Memo cards expose outbound publication. Their Telegram paper-plane and X icons sit
immediately after the `public` label in the card header; private cards render no publication control.

## Knowledge interaction

Knowledge uses the shared frame for its index, article, and editor. Article navigation and editing
actions wrap in the shared header; the collapsible table of contents stays in the content column.
Article editing uses a rich-text toolbar and an explicit
Markdown source mode; unsupported rich-text syntax opens in source mode without rewriting content.
The selected article, draft fields, and pending save survive switching pages during the session.
Saving preserves subsequent edits and updates the article revision for the next submission. Cancel
discards the current edit. Session drafts end when the WebView reloads or the application restarts.

Compiled GitHub and stock embeds use an Innei-inspired editorial card treatment: warm paper, fine
outlines, light shadows, compact line icons, and one muted accent. The compiler emits the styles.
Architecture canvases use transparent authored SVG with Claude-style rounded nodes, curved
connectors, compact type hierarchy, and semantic muted color groups. Storyboard canvases use
irregular paths, offset double strokes, round-ended scribbles, and a handwritten CJK-capable font
stack. They remain transparent and frame-free so the consumer theme stays visible.

## Moment interaction

Moment opens on its original masonry gallery: one column below a 640px window, two below 1024px,
and three otherwise. Sidebar resizing changes column widths without regrouping photos. Upload and
Filter are compact icons beside the Moment page title. Upload enters a focused upload view;
Filter toggles the panel and retains its active state. The filter panel owns the complete tag
index, Any/All matching, and date order, with active tags summarized as removable chips. Tag reads
show loading or retryable errors without replacing the gallery or clearing a settled index.

Selecting a photo opens an application-modal viewer with keyboard navigation, contained scroll
focus, and a toolbar for sharing, editing, deleting, and closing. The preview remains visible until
the original is decoded and ready to replace it. Upload combines preview, title, description, tags,
date, and coordinates; errors stay within the active surface while settled gallery content remains
visible.

## Music interaction

Music opens the selected provider's collection: Spotify Liked Songs or QQ Music Daily 30. Switching
providers restores that collection's settled list. A successful song selection opens the player;
a failed selection keeps the current song visible and reports the error. The lower-right back action
returns to the prior page, while primary Music navigation opens the collection.

The player fills the available canvas height with vinyl, a lyric subtitle, metadata, transport
controls, and a seekable timeline. The record area absorbs extra height; short windows scroll the
complete player. Synced lyrics follow timestamps, and plain lyrics advance proportionally.
Previous and Next follow the collection, with sequential, repeat-one, and shuffle modes owned by Rust.

Playback controls serialize pending actions. Polling pauses during an action, and stale collection,
playback, or lyric responses cannot overwrite newer selections. Reduced-motion preferences disable
vinyl and subtitle animation while retaining playback state.

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
Settings as a full navigation tab. Consumer destinations appear only when their corresponding
credentials are configured; Newspaper shares Knowledge configuration, Music accepts either provider,
and Inbox requires ntfy. Dashboard and Settings remain available while configuration loads. The
sidebar has no separate Connections inventory. Removing the active destination’s configuration
returns the shell to Dashboard. An editable local profile badge anchors the left side of the footer
beside its three controls; selecting it exposes a compact floating name and avatar editor above the
footer. The editor receives focus when opened and collapses when focus leaves the badge and popover. The
square-cropped avatar and display name persist locally and do not imply an application session. On
desktop, the sidebar's right edge supports pointer dragging and keyboard resizing within bounded
widths, and remembers the chosen width. Below 160px it hides navigation and profile text; the 64px
minimum retains navigation icons and only the avatar in the footer. Inbox, App Lock and theme
controls are hidden until the sidebar expands. Navigation buttons
keep accessible names and hover titles, and the profile editor opens beside the compact rail.
Arrow keys resize in 8px steps; Home and End select the minimum and maximum widths. Mobile keeps
the full-width labeled drawer regardless of the saved desktop width.
Profile editing and updater presentation each own their complete interaction state in layout
components; the shell owns sidebar sizing, navigation and modal exclusion.

The footer separates destinations from immediate actions
without a visual divider: Inbox remains a destination and shows a small status dot while unread
notifications exist, while App Lock and theme are immediate actions.
When no password exists, the lock control routes to Settings; otherwise it immediately makes the
complete application shell inert and shows one opaque unlock surface. The unlock form receives focus,
reports an incorrect password in place, and reveals no underlying content. Settings owns password
creation, replacement, and removal, and describes App Lock as a privacy screen rather than encryption.

## Settings interaction

Settings uses horizontal categories above a single content column, with consistent card headers,
field spacing and action footers. Narrow layouts scroll the categories and stack labels above inputs.
Category buttons use compact insets and name each section without repeating a secondary heading.
Category switches keep forms mounted to preserve drafts and pending operations. Settings and game
connections share one ConfigurationBadge treatment for configured credentials, environment sources,
and authorization state.

QQ Music connection owns its QR dialog and pending login lifecycle. Closing or leaving Settings
invalidates pending QR responses and clears its timer; a late result cannot dismiss a newer dialog.
Settings accepts only the latest configuration read.

Each credential form tracks its own saved values and pending operation. Save is enabled only when
required fields are complete and the form differs from its saved values. Saving one form leaves
other forms available and preserves their drafts when configuration refreshes. Successful saves
record the submitted values, so edits made during a request remain unsaved; failures remain retryable.
Telegram credential saving and account authorization serialize within the same provider.

## Game interaction

Settings presents separate miHoYo/Skland QR connections, saved miHoYo accounts with one selector
per game, and a Steam credential form. QR dialogs center a scrollable card and paint their overlay
on the dialog surface so themes work across WebViews. Escape/Close cancels login; expired codes can
be renewed. The shared `Select` supports keyboard navigation, visible selection, Escape dismissal,
and outside-click closing. Each game retains its own account selection; removing a login preserves
its archive. Steam stacks SteamID64 and the concealed API key in full-width rows, prefills saved
values, and preserves edits during reads and saves. Unchanged forms cannot be submitted.

Game cards span all twelve Dashboard tracks. Daily metrics sit beside pull charts and manual archive
actions, separated by a quiet divider. miHoYo content areas are 14rem high with independent scrolling;
narrow cards stack the sections. Refresh and verification controls remain reachable as content grows.

Daily metrics use two compact columns. Task progress with up to five steps uses filled and outlined
stars, with an accessible step count and the original numeric value in its tooltip. Genshin uses
this treatment for commissions, expeditions and weekly discounts; Star Rail uses it for assignments
and daily training, where each star represents a 100-point milestone. Power and reserve amounts
retain numeric values. Dashboard and Dynamic Island render the same task treatment.

Daily refresh is an icon-only action with a tooltip and accessible label. Genshin and Star Rail
verification windows show loading, submission, and completion states, then direct the user to close
the window and refresh the card. ZZZ uses its official record page, whose login action opens
Settings → Games. Closing verification preserves the displayed daily state. Game status and
verification copy, including Geetest, use English.

Archive rings include textual counts and accessible labels. Star Rail uses a doughnut chart for
pools' shares of official total pulls, semantic-color legend markers, and counts since the last
five-star. It does not imply a rarity breakdown unavailable from that API. Six compact pool rows sit
beside the chart; coverage guidance and five-star records share a disclosure below it. Expanded
records use the panel's scroll area without a nested scroller. Archive refresh remains icon-only.

Steam places account and statistics beside recent activity and the five most-played games. Presence
sits beside the account name; update time shares the header with refresh. Summary values are
right-aligned and do not wrap, with supporting text below. Compact single-line game rows keep the
strip shallow, link to game store pages, and expose lifetime hours on hover in the recent list.
There is no profile link.

## Newspaper and Inbox

Newspaper presents the latest Programmer Daily and Personal Daily articles without an archive.
Page-edge arrows turn the reading surface like an album and start the selected edition at the top;
background refreshes preserve the reading position. Reduced-motion preferences disable the turn
animation. The reading surface retains its warm paper background, generous inset, compact edition
line, large serif masthead, and original article hierarchy. A shared Newspaper H1 sits above the
paper at the same position as its loading header. Inside the paper, the issue title is an H2 with
the original masthead styling; the paper does not inherit the shared page-title scale.

The Inbox control opens notification content independently from other views. An unreadable local
store shows an error instead of an empty inbox; the rest of the application remains available.
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
- Compact actions such as Manage, Check in, Undo, Add, and Remove prefer icons over visible text.
  Each icon button needs a descriptive tooltip and accessible name; toggles expose their active state.
- Muted text and status colors must remain readable in both themes.
- Motion should use the shared duration tokens and remain limited to meaningful feedback.
