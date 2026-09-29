# Anki Preferences — Support Matrix

This document compares the preferences described in the [Anki manual](https://docs.ankiweb.net/preferences.html) — reconciled against the **in-app descriptions** — and decides which ones Anjuman's **server** owns vs. which are client-side, form-factor-specific, or Anki-specific.

Legend:

- ✅ supported (server persists/consumes it)
- 🟡 partial / differs
- ❌ not supported (but a server-side concern — a real gap)
- ⚪ not the server's concern — client-side or N/A

## Scope philosophy

Most of Anki's preferences do **not** belong in the server:

- **Client-side** — theme, playback toggles, on-screen count/next-review-time
  display, distraction modes. The client owns these (stage 4) and may store them
  where it likes (local storage); the server has no role.
- **Form-factor-specific** — keyboard shortcuts/answer keys, "spacebar also
  answers", `Style` (Anki vs Native). These are resolved by each platform shell
  (desktop vs. mobile — we may consult AnkiDroid for mobile conventions later);
  they are **not** cross-user server state.
- **Anki-specific / "do nothing"** — video driver, `Style`, reset-window-sizes,
  update/addon checking, third-party services, experiments. We deliberately do
  not reproduce them.
- **Server-side** — the small set that actually changes *shared scheduling
  behaviour* and therefore must live in the DB, keyed per user. These are the
  only preferences we implement (see ROADMAP stage 3).

The upshot: **the server owns only three preferences today** — `day_start_hour`
("next day starts at"), `learn_ahead_seconds` ("learn ahead limit"), and
`timebox_time_limit` (a future timeboxing feature) — plus the CRUD endpoint that
edits them.

## Appearance

| Anki feature | Status | Notes |
|---|---|---|
| Language | ⚪ | Client concern (i18n). |
| Video driver (OpenGL/Vulkan/Software) | ⚪ | Rendering is the client's job; no server role. **Decision: drop.** |
| Check for program updates | ⚪ | We use each platform's native update mechanism. **Decision: drop.** |
| Check for addon updates | ⚪ | No addons; a possible future stage. **Decision: drop.** |
| Theme (Dark/Light/Follow System) | ⚪ | Client concern. |
| Style (Anki vs Native) | ⚪ | Anki-client-specific. **Decision: drop.** |
| User interface size | ⚪ | Fall back to the OS / accessibility defaults; users manage this via the OS. **Decision: drop.** |
| Reset window sizes | ⚪ | Anki-client-specific button. **Decision: drop.** |
| Hide top/bottom bar during review | ⚪ | Client concern (form-factor/distraction). |
| Reduce motion | ⚪ | Client concern. |
| Minimalist mode | ⚪ | Client concern. |

## Review

### Scheduler (server-side)

| Anki feature | Status | Default | Notes |
|---|---|---|---|
| Next day starts at | ✅ | 4 | `user_preferences.day_start_hour` (whole hours 0–23). Drives day boundary for limits, burial expiry, and study bucketing. |
| Learn ahead limit | ✅ | 20 (min) | `user_preferences.learn_ahead_seconds` (default 1200 s). In-app it's **minutes, 0–100, default 20**. |
| Timebox time limit | ❌ | 0 | Int minutes, 0–9999, default 0 (`0` = disabled). Genuine server-side gap — depends on implementing timeboxing itself (no timebox feature yet). |

### Review (client-side / form-factor)

| Anki feature | Status | Notes |
|---|---|---|
| Show play buttons on cards with audio | ⚪ | Client + audio support (stage 4). |
| Interrupt current audio when answering | ⚪ | Client + audio support (stage 4). |
| Show remaining card count | ⚪ | Counts are already returned; the *display* toggle is client. |
| Show next review time above answer buttons | ⚪ | Intervals are already returned; *display* is client. |
| Spacebar/Enter also answers card | ⚪ | Form-factor-specific (desktop). |
| Generate LaTeX images (security) | ⚪ | Client + renderer concern. |

### Answer keys (form-factor)

| Anki feature | Status | Default | Notes |
|---|---|---|---|
| Again / Hard / Good / Easy | ⚪ | 1 / 2 / 3 / 4 | Keyboard mapping — form-factor-specific; client owns it. |

## Editing

| Anki feature | Status | Notes |
|---|---|---|
| Editing conveniences | ⚪ | Client concern. |
| Browsing / default search / accent-insensitivity | ⚪ | Search behaviour — see stage 6 (search) rather than preferences. |

## Syncing / Backups / Third-party services / Experiments

| Anki feature | Status | Notes |
|---|---|---|
| Sync / AnkiWeb account | ⚪ | The server is always the source of truth; there is no device-sync layer. **Decision: drop** (offline sync, if ever, is a separate feature). |
| Backups | ⚪ | Handled at the DB level, not exposed to users. **Decision: drop.** |
| Third-party services (AnkiHub, …) | ⚪ | Anki-specific. **Decision: drop.** |
| Experiments (Svelte editor, …) | ⚪ | Anki-specific. **Decision: drop.** |

## Summary of the biggest gaps

1. **Preferences CRUD** — no `GET/PATCH /preferences` endpoint yet (the two
   persisted columns are read directly, but users can't edit them).
2. **Timebox time limit** — the missing server-side preference, gated on
   implementing timeboxing.

## Key architectural differences vs. Anki

- **Only two persisted server-side preferences** (`day_start_hour`,
  `learn_ahead_seconds`); a third (`timebox_time_limit`) is planned. Everything
  else in Anki's preferences dialog is client-side, form-factor-specific, or
  deliberately dropped as Anki-specific.
- **Day boundary is UTC-anchored** — "next day starts at" stores only an hour
  (`day_start_hour`) with no per-user timezone, so the boundary is computed
  against UTC (per-user timezone is a possible future refinement).
- **Preferences are not yet user-editable** — the `user_preferences` columns
  have defaults and are read by `study.rs`, but there is no endpoint to
  read/update them (unlike deck options, which have full CRUD).
- **Form-factor preferences are intentionally deferred** to the client shells;
  we may consult AnkiDroid for mobile conventions rather than invent our own.
