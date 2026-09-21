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

## Style contracts

Apply these rules to Svelte templates, scoped styles, shared CSS, and class helpers. They adapt
the design-system checks from [shadcn lint](https://github.com/shadcn-ui/lint) to this workspace;
the plugin is not installed and these are review requirements, not automated lint coverage.

- **Semantic colors:** use declared `--color-*` tokens, including for SVG and focus indicators.
  `transparent`, `currentColor`, inheritance, and `color-mix` with semantic colors are valid.
  Do not copy another system's token names: this theme uses `accent`, `error`, and `background`,
  not `primary`, `destructive`, or `card`. Raw color values belong in `palette.css`.
- **Component appearance:** select a shared component's variant and size before overriding its
  classes. Callers may position components with margin, width, alignment, and grid placement.
  Repeated color, border, weight, or padding overrides belong in the component's typed contract.
  Badge's `accent-outline` variant owns the outlined accent treatment; `size="sm"` owns its compact
  date-label sizing. Interaction-specific hover states may stay with the caller.
- **Static styles:** keep fixed appearance in scoped CSS or complete class literals. Use inline
  styles for measured positions, progress, media aspect ratios, and data-selected semantic chart
  colors. Local CSS custom properties carrying these values are not global design tokens.
- **Intentional values:** reuse radius, shadow, font-family, heading, and duration tokens.
  Layout dimensions, responsive breakpoints, chart geometry, circular `50%` radii, and decorative
  rings may remain explicit. Do not create a global token for every one-off measurement or round
  existing dimensions merely to eliminate arbitrary values. Preserve animation timing when
  replacing literals with named duration tokens.
- **Resolvable classes and tokens:** Tailwind utilities must be complete strings or a finite map
  of complete strings, never fragments such as `bg-${color}`. Scoped class names may be computed
  when their selectors are explicitly defined. Every referenced design token must exist in the
  theme; a plausible name does not make an undefined custom property valid.

Review exceptions at their owning component, preserve light/dark contrast and reduced-motion
behavior, and avoid broad `!important` or descendant overrides of shared component internals.

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
The check has an accessible description; future days stay unmarked. A past or current day with
no tasks or configured habits counts as complete.

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
Compact wings show the selected widget's icon and Planner's open-task count when available.
Expanded widgets omit their outer card frame; bounded content scrolls with hidden scrollbars.
The Dashboard's pinned-widget strip appears only while editing. Embedded Planner uses Tasks,
Calendar and Habits tabs, preserving input when switching; the selected date stays visible.

Pinning selects a saved Dashboard placement. Hover expands after a short dwell; pointer exit allows
brief re-entry, while focused controls keep it open. Click or keyboard activation opens immediately
and focuses Close. Escape, Close and focus loss dismiss; Escape and Close restore trigger focus.
Frame changes preserve the top anchor and respect Reduce Motion. Losing focus must not reactivate
the window. App Lock closes it; other platforms omit pinning.

## Window and theme

Retain the macOS native title bar and drag behavior. Theme changes update native and WebView
appearance directly; avoid page-level or independent control color transitions. The shell owns one
back-to-top action tied to its main scroller.

The sidebar keeps navigation primary, with Settings as a full destination. Configured credentials
gate consumer destinations; removing the active destination's configuration returns to Dashboard.
Its resizable desktop rail retains accessible icon names when collapsed; mobile keeps the labeled
drawer. The local profile editor stores presentation only and does not imply an authenticated account.

A This device terminal entry sits immediately above the sidebar’s Tailscale list and stays available
when discovery fails. It uses the same terminal frame without remote login controls.
Sidebar Tailscale devices show accessible connection statuses. Selecting one opens a terminal
filling the main pane, with username and reconnect/disconnect controls; navigation preserves its
contents until that device is selected again. Each sidebar device click starts a fresh connection,
including after an idle timeout. xterm.js handles input and screen-reader support, using semantic `--color-terminal-*`
roles for theme-consistent canvas and ANSI colors.

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
Memo editors constrain content-sized textareas to their container and wrap unbroken URLs.

## Knowledge interaction

Index, reader, and editor share the page frame. The index renders metadata before article bodies
load. Visible entries, hover, focus and touch warm destinations; pending clicks keep titles stable
and expose busy state. Detail failures preserve the index for retry. Newspaper loads the active
edition and preloads the other on arrow intent, rejecting stale presentation updates.

The reader toolbar groups contents, copying the canonical UUID URL, editing, and returning. Actions
align with the article title. Contents entries scroll the main reader with heading clearance and
respect Reduce Motion; pointer selection survives menu focus changes. Outside clicks and Escape
close the menu. Copying reports success only after the clipboard write completes and shows failures.

Article-list rows pair document thumbnails with resolved titles and summaries. Internal references
open Knowledge and retain a bounded reading history; Back returns to the previous article before
the index. Chapter links locate headings after the destination mounts. Failed navigation preserves
the current article. Ordinary external links open the system browser.

Rich editing includes explicit Markdown mode; unsupported syntax opens there without rewriting it.
Session drafts survive navigation, saving preserves edits made during the request, and unfinished
edits block switching articles. Existing-article visibility is staged with content until Save;
Cancel discards it. Changing the select alone never writes to the server.

## Markdown presentation

Shortcode images align with text: emoji occupy 2rem squares and stickers retain their aspect ratio
within 6rem; Combot’s 128px previews are capped at 4rem. They load lazily with descriptive alt text and without referrers.

Compiled embeds use restrained paper, outlines, shadows and accents. Alignment applies to individual
cards or whole article lists: left/right cap width at 32rem and align to that edge, narrow centers
the same bounded width, and wide fills the available container subject to type-specific caps.

Attributed quotes preserve multiline text with a source footer. Git diffs use a keyboard-scrollable
code region, visible plus/minus markers and semantic success/error colors; headers remain muted.
Annotations retain the sentence with a semantic-color highlight and a visible note below it. The
note has a matching border and may link to a source; text wraps without script-driven positioning.

Audio and video use responsive native controls, optional captions and video posters, and never
autoplay. Videos without a poster request an opening-frame preview; playback starts with Play.

Architecture and storyboard canvases remain transparent and frame-free. Structured engineering
diagrams wrap labels and grow nodes and edge anchors with the text, then group parallel nodes into dependency columns. At most 640px of actual diagram width,
including narrow alignment, selects a vertical layout with at most two nodes per row. Generated
engineering diagrams have no maximum height. Authored SVG retains its geometry and a 42rem display
height cap. Syntax and compiler rules belong to [Markdown](MARKDOWN.md).

## Moment interaction

The masonry gallery uses one column below 640px, two below 1024px, and three otherwise. Sidebar
resizing changes widths without regrouping. Filters retain visible removable selections and settled
indexes. Photo viewing is application-modal with keyboard navigation and contained focus; the
preview remains until the original decodes. Upload errors preserve the active form and gallery.
Selecting a photo prefills capture time and GPS from local EXIF; missing or invalid GPS stays empty
with an explanation. Reads preserve edited fields and discard results after removal or closing.
Publish waits for metadata. Unedited capture time retains the camera's wall clock and offset.

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
Article cards open Knowledge through the shared reader navigation; ordinary external links open
the default browser and same-article fragments stay in the reader. Opening failures appear beside
the article.

Inbox is independent of Dashboard. Unreadable storage displays an error, never a false empty state.
Replayed notifications populate history; only new live messages may trigger system notifications.

## Accessibility

Use native interactive elements, visible keyboard focus, and descriptive names for icon actions.
Toggles expose state; progress exposes a name and numeric bounds. Immediate errors use an alert role.
Keep muted text readable in both themes, and communicate status beyond color. Dialogs contain focus
and return it when dismissed. Skeletons hide decorative content while the region announces loading.
Use shared motion durations and disable nonessential animation for reduced-motion preferences.
