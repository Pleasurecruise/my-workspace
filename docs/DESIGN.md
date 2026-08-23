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

## Accessibility

- Interactive elements use native buttons and inputs.
- Progress indicators expose `role="progressbar"`, a provider-specific accessible label, and numeric
  bounds.
- Errors that require immediate attention use an alert role.
- Icon-only controls require an accessible label.
- Muted text and status colors must remain readable in both themes.
- Motion should use the shared duration tokens and remain limited to meaningful feedback.
