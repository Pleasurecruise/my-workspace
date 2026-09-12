# Design System

This document defines shared visual and interaction rules. Provider protocols belong to
[Dashboard](DASHBOARD.md); durable state and engineering rules belong to
[Persistence](PERSISTENCE.md) and [Code style](STYLEGUIDE.md).

## Token architecture

`palette.css` defines physical colors, `tokens.css` assigns semantic roles, and `theme.css` maps
those roles to utilities. Components consume only `--color-*`, `--font-*`, `--radius-*`,
`--shadow-*`, and `--duration-*` tokens. `--palette-*` is allowed only in `tokens.css`.
Do not embed raw colors or framework palettes in views. Static decisions belong in styles;
computed dimensions and progress values may use inline styles.

## Component ownership

`packages/ui` owns reusable primitives and tokens; desktop features own composition. Promote a
component only for a stable shared responsibility, not to shorten imports.

Shared components follow [shadcn-svelte](https://shadcn-svelte.com/docs) conventions with locally
owned styling and interaction. Do not mix packaged headless runtimes into individual controls.
Contracts include Svelte 5 snippets, bindable element `ref`, native attributes, `cn` overrides,
component-specific `data-slot`, keyboard focus, and disabled/invalid states. Visual roles use
Vesper's semantic tokens and compact scale.

Select provides keyboard navigation, typeahead, dismissal, and visible selection. Checkbox uses
native checked/indeterminate semantics; controlled saves retain the accepted value until updated.
The Vesper-specific `SortableList` offers pointer dragging and keyboard ordering through a dedicated
handle, cancellation, retained focus, and live announcements. Pending saves disable handles; only
successful saves announce a new position. Consumers own saved order and errors.

## Page layout and typography

One centered `page.css` frame serves loading, reading, editing, and error states. Its maximum width
is 84rem including 2rem side padding; below 768px the padding is 1rem. Headers retain one baseline
across states, with H1, optional description, and wrapping actions. Reserve the main scrollbar gutter
to prevent horizontal shifts. Content responds to its available width, including sidebar resizing.

Use the shared semantic heading scale and serif page title. Dashboard cards, Moment, and Newspaper
may own internal typography through `data-content-typography`; their page titles still use the
shared scale. Newspaper's 58rem paper surface remains an internal composition. Do not place
containment around viewport-fixed dialogs or override page spacing to compensate for local content.

## Dashboard layout

Use a twelve-track canvas with 0.5rem gaps and generally 0.75rem card insets. Saved spans and order
remain stable as the window narrows. Edit mode exposes drag, remove, and pin actions; Add Widget uses
a category rail, list, and configuration preview. Invalid widgets keep their position and show
escaped configuration with expandable errors, while valid cards remain usable.

Daily Planner fills one row with Calendar, Todo, and habits. Fixed-height content scrolls within
lists or details; compact surfaces stack the sections. The Sunday-first calendar controls the
shared date. Today's indicator, selection, and the small completed-day check remain visually distinct.
The check has an accessible description; empty and future days stay unmarked.

Todo titles open details without changing card size. Completion uses the row checkbox; sorting uses
a separate handle. Manual items support editing with Save and Cancel; imported content is read-only.
The opt-in daily carry-forward checkbox belongs at the very bottom of details, below the scroll
area, with small muted explanatory text. Its tooltip explains repetition until completion or opt-out.
Habit check-in/undo uses a distinct icon action, with disabled future actions and an explanation.

Spending is a separate full-width card sharing the selected date. Entry and dated records sit beside
monthly charts; compact surfaces stack them. Chart changes preserve drafts and height. Labels and
amounts must explain donut segments and daily bars without relying on color. Empty months are explicit.

Provider cards prioritize the current value and supporting context. Telemetry trends are session
history; storage offers system settings for category details. Service status shows affected names,
not only a colored percentage. GitHub notification failures remain inside their section. Preserve
settled content during background refresh; initial placeholders belong only to the affected surface.

## Dynamic Island

The macOS island uses a continuous black surface with outward shoulders and rounded lower corners,
following the [Atoll](https://github.com/Ebullioscopic/Atoll) reference. It adapts to the notch,
anchors at the screen's top center, and uses scoped dark semantic tokens independently of the app.
Expanded content is bounded and scrollable; embedded widgets omit their outer card frame.

Pinning selects a saved Dashboard placement. Pointer or keyboard activation expands; Escape, Close,
or leaving without pointer/focus collapses. Frame changes preserve the top anchor and respect Reduce
Motion. Losing focus must not reactivate the window. App Lock closes it; other platforms omit pinning.

## Window and theme

Retain the macOS native title bar and drag behavior. Theme changes update native and WebView
appearance directly; avoid page-level or independent control color transitions. The shell owns one
back-to-top action tied to its main scroller.

The sidebar keeps navigation primary, with Settings as a full destination. Configured credentials
gate consumer destinations; removing the active destination's configuration returns to Dashboard.
Its resizable desktop rail retains accessible icon names when collapsed; mobile keeps the labeled
drawer. The local profile editor stores presentation only and does not imply an authenticated account.

App Lock makes the entire shell inert behind an opaque focused unlock surface. Settings describes
it as a privacy screen, not encryption. Update installation requires an explicit action after showing
version and release notes; progress and failures remain visible without obstructing normal startup.

## Settings interaction

Use horizontal categories and a single content column with consistent fields and footers. Categories
preserve mounted drafts. Narrow content stacks labels and wraps actions. Configuration badges use
one treatment for saved values, environment sources, and authorization state.

Each form owns its saved values and pending action. Save requires a valid changed form; successful
saves accept submitted values while preserving edits made during the request. Other forms remain
usable. Closing login dialogs cancels their interaction, and late responses cannot reopen or replace
a newer dialog. Conceal credential fields and keep failures retryable.

## Memo interaction

Composer and editor focus belongs to the containing surface. Session drafts survive navigation;
saves clear only unchanged submitted text. Switching the active memo saves the current edit first,
while Cancel explicitly discards it. Tag completion respects input-method composition.

Tags and filters remain removable independently of refreshed indexes. Pinned entries have a collapsed
section in the unfiltered feed. Internal Memo links locate and reveal the target in the application;
external links use the system browser. Archive uses a month-grouped reading view; Favorites retains
regular cards and editing. Only public cards expose outbound Telegram and X actions.

## Knowledge interaction

Index, reader, and editor share the page frame. The index renders from metadata before article
bodies load. Visible entries preload after the index renders; hover, focus and touch also warm destinations.
Pending clicks keep titles stable and expose busy state to assistive technology. Detail failures keep the index available for retry.
Newspaper loads only the active edition and preloads the other edition on arrow intent. Switching
editions cancels stale presentation updates; loading and failure states are distinct from unpublished
editions. The reader toolbar groups table-of-contents navigation, copying the canonical article URL, editing
and returning to the index. Contents entries scroll the current main reader with heading clearance
and respect Reduce Motion. Pointer selection survives menu focus changes; outside clicks and Escape
close the menu. Copying shows success only after the clipboard write completes and reports failures.

Article shortcuts use compact, keyboard-accessible title links and automatically resolved summaries.
Internal references open Knowledge and retain a bounded reading history; Back returns to the
previous article before returning to the index. External targets open the system browser. Failed navigation
preserves the current article. Rich editing includes explicit Markdown mode; unsupported syntax
opens there without rewriting it. Session drafts survive navigation, saving preserves edits made
during the request, and unfinished edits block switching articles.
Compiled embeds use restrained paper, outlines, shadows, and accents. Audio and video embeds use
responsive native controls, optional captions and video posters, and never autoplay. Videos without
a supplied poster request an opening-frame preview; playback starts through the play control. Architecture and storyboard
SVG remain transparent and frame-free, with their authored semantic hierarchy intact.

## Moment interaction

The masonry gallery uses one column below 640px, two below 1024px, and three otherwise. Sidebar
resizing changes widths without regrouping. Filters retain visible removable selections and settled
indexes. Photo viewing is application-modal with keyboard navigation and contained focus; the
preview remains until the original decodes. Upload errors preserve the active form and gallery.

## Music interaction

Collection navigation preserves the selected provider's settled list. A successful track selection
opens the player; failures retain the current song. The player balances vinyl, lyrics, transport,
and seeking within available height, scrolling in short windows. Playback controls serialize pending
actions. Reduced motion disables decorative rotation and lyric transitions. Provider behavior and
media ownership are documented in [Music](MUSIC.md).

## Game interaction

Daily metrics and pull archives share full-width cards; narrow surfaces stack sections. Keep refresh
and verification reachable within scrolling content. Progress stars and archive charts include
textual counts and accessible labels, without implying data the provider does not supply.
Settings keeps account selection distinct per game; closing authorization or verification preserves
settled content. Game status and verification copy use English. [Games](GAMES.md) owns provider flows.

## Newspaper and Inbox

Newspaper presents the latest two editions on a warm paper surface with a serif masthead and original
article hierarchy. Index lookup and initial detail compilation share one continuous Newspaper
skeleton; the finished edition replaces it directly. Background refresh retains the settled paper
without adding a second loading indicator. Edition changes start at the top; background refresh
preserves reading position.
Page-turn motion respects Reduce Motion. Editions stay out of the Knowledge index and Inbox.
Newspaper and Knowledge article web links open in the default browser, preserving the reader;
fragment links remain within the article. Opening failures appear beside the article.

Inbox is independent of Dashboard. Unreadable storage displays an error, never a false empty state.
Replayed notifications populate history; only new live messages may trigger system notifications.

## Accessibility

Use native interactive elements, visible keyboard focus, and descriptive names for icon actions.
Toggles expose state; progress exposes a name and numeric bounds. Immediate errors use an alert role.
Keep muted text readable in both themes, and communicate status beyond color. Dialogs contain focus
and return it when dismissed. Skeletons hide decorative content while the region announces loading.
Use shared motion durations and disable nonessential animation for reduced-motion preferences.

Article-list rows pair document thumbnails with resolved titles and descriptions, following workspace’s reading layout. Return and edit actions align vertically with the article title. The bounded reading trail records article navigation in order. The Knowledge reader handles their internal article destinations without opening the system browser. Existing-article editors stage the Visibility select until Save; switching or cancelling never writes it immediately.
