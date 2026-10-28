# Carve-out (core agent → shell agent): card night mode (Anki-faithful), phase 2 of dark mode

**From:** core agent
**To:** shell agent
**Status:** resolved (shell `client/shells/react/app/components/study.tsx`)

## Context

Phase 1 (UI dark mode via `ThemeSelector`/`useTheme`) is done in the shell. Phase
2 is making the **study cards** honour dark mode the way Anki does. This is
entirely the shell's scope — **no core/server/contracts change is needed**.
Everything needed either is already delivered (`StudyCardView.styling`) or is
shell-local.

## The Anki model (authoritative)

Anki's card night mode is a **three-layer** system, toggled by adding a
`nightMode` class to the card's page (Anki puts it on `body`/`html`) when night
mode is on:

1. **Built-in base** (always applied, independent of the card author) — from
   Anki's reviewer stylesheet, this gives every card a dark canvas + light
   foreground so it is *never* left blindingly white:

   ```css
   /* explicit nightMode definition required
      to override default .card styling */
   body.nightMode {
       background-color: var(--canvas);
       color: var(--fg);
   }
   ```

2. **Author overrides** (opt-in) — the note type's "Styling" block *may* contain
   rules that further tune specific elements on top of the base:

   ```css
   .card.nightMode { background-color: #555; }
   .nightMode .myclass { color: yellow; }
   ```

3. **Built-in media inversion** (always applied) — specific content types, not
   arbitrary images:

   ```css
   .nightMode .latex { filter: invert(100%); }
   .nightMode img.drawing { filter: invert(1) hue-rotate(180deg); }
   ```

Reference: <https://docs.ankiweb.net/manual/templates/styling#night-mode>,
<https://docs.ankiweb.net/ankimobile/night-mode.md>, and Anki's own
`ts/reviewer/reviewer.scss`.

`--canvas` / `--fg` are Anki's app-wide theme variables; Anjuman should supply
its **own** dark-theme equivalents (see below), not Anki's literal hex values.

## What to change

In `client/shells/react/app/components/study.tsx` (all shell-local):

1. Read the theme via `useTheme()` and resolve `isDark` (reuse `theme-provider`'s
   resolution of `light`/`dark`/`system` + the system preference).

2. Thread `isDark` into `StudyCard` and pass it to `cardHtml`.

3. Update `cardHtml` to:
   - add `class="nightMode"` to `<html>` when dark;
   - inject a **built-in night-mode base stylesheet** (our equivalent of
     `body.nightMode { background-color/color }`) *after* the author `styling`, so
     it can supply the dark default without clobbering author `.nightMode`
     overrides (author rules that come later in the same `<style>` win; simplest
     correct ordering is: author `styling` first, then the base block).

   Sketch (values are placeholders — use your app's dark theme tokens):

   ```ts
   const NIGHT_MODE_BASE = `
     body.nightMode { background-color: #1f1f1f; color: #e5e5e5; }
     .nightMode .latex { filter: invert(100%); }
     .nightMode img.drawing { filter: invert(1) hue-rotate(180deg); }
   `;

   function cardHtml(body: string, styling: string, nightMode: boolean): string {
     const htmlClass = nightMode ? ' class="nightMode"' : "";
     const nightCss = nightMode ? `<style>${NIGHT_MODE_BASE}</style>` : "";
     return `<!DOCTYPE html><html${htmlClass}><head><meta charset="utf-8" />`
       + `<style>${styling}</style>${nightCss}</head><body>${body}</body></html>`;
   }
   ```

## Parity notes

- **The base fallback is required.** Do not ship only the `nightMode` class and
  rely on author CSS — that would leave cards without `.nightMode` rules rendering
  white in dark mode, which is *not* faithful. The `body.nightMode` base (dark
  canvas + light text) is part of Anki's behaviour.

- **Author `.nightMode` rules must still be able to override the base.** Put the
  author `styling` before the base block, and/or give the base low specificity so
  `.card.nightMode { … }` wins where the author intends.

- **Do not invert arbitrary images.** Only LaTeX (`.latex`) and drawings
  (`img.drawing`) are inverted by Anki. A blanket `img { filter: invert(…) }`
  would corrupt normal images and is not Anki behaviour.

## Scope / boundary

- Only `client/shells/react/` (the shell's domain). Do not touch `contracts/`,
  `server/`, or `client/shared/` — the note type's `styling` is already delivered;
  nothing is missing on the core side.
- Theme is client-corner per `docs/support/preferences.md` ("Theme (Dark/Light/
  Follow System) | ⚪ | Client concern"), so keep it shell-local.

## Validation

- `cargo build` in the client workspace (shell compiles against unchanged `shared`).
- Manual: in dark mode —
  - a note type with `.card.nightMode { … }` shows that override;
  - a note type with **no** `.nightMode` rule still shows the dark base (dark
    background, light text);
  - LaTeX / drawing images (if present) invert.
- Commit with Conventional Commits scope `shell/react`.
