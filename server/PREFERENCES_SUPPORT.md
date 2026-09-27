# Anki Preferences — Support Matrix

This document compares the preferences described in the [Anki manual](https://docs.ankiweb.net/preferences.html) against what Anki Classroom supports, and captures where we differ.

Legend:

- ✅ supported
- 🟡 partial / differs
- ❌ not supported
- ⚪ not applicable (client-side / backend has no role)

> Note: many Anki preferences are **client-side** (theme, window sizes, keyboard shortcuts, playback behaviour) and therefore live in the Flutter frontend rather than the backend. This document marks them ⚪ and focuses the "supported" analysis on behaviour the backend actually controls.

## Appearance

### General

| Anki feature | Status | Notes |
|---|---|---|
| Language | ⚪ | Frontend concern. No backend involvement. |

### User Interface

| Anki feature | Status | Notes |
|---|---|---|
| Theme (dark/light/auto) | ⚪ | Frontend concern; night-mode card styling may require template work (same caveat as Anki). |
| User interface size | ⚪ | Frontend concern. |
| Reset window sizes | ⚪ | Frontend concern. |
| Video driver | ⚪ | Desktop client concern (Anki-specific; not relevant to Flutter). |

### Distractions

| Anki feature | Status | Notes |
|---|---|---|
| Hide bars during review | ⚪ | Frontend concern. |
| Minimalist mode | ⚪ | Frontend concern. |
| Reduce motion | ⚪ | Frontend concern. |
| Native vs Anki theme | ⚪ | Frontend concern. |

## Review

### Scheduler

| Anki feature | Status | Notes |
|---|---|---|
| Next day starts at (default 4 AM) | ✅ | `user_preferences.day_start_hour` (default 4). Used for daily limits, burial auto-expiry, and study bucketing. See caveats below. |
| Learn ahead limit (default 20 min) | ✅ | `user_preferences.learn_ahead_seconds` (default 1200). Used as a fallback when no actually-due cards remain. |
| Timebox time limit | ❌ | Not implemented (no timeboxing). |

### Review

| Anki feature | Status | Notes |
|---|---|---|
| Show play buttons on cards with audio | ❌ | No audio support yet. |
| Interrupt current audio when answering | ❌ | No audio support yet. |
| Show remaining card count | 🟡 | Card counts exist (`counts`) but any "hide count" toggle is frontend-only. |
| Show next review time above answer buttons | 🟡 | Intervals are returned (`predicted_intervals`, `applied_interval_secs`); whether/how to display is frontend. |
| Spacebar / Enter also answers card | ⚪ | Frontend concern. |

## Editing

### Editing

| Anki feature | Status | Notes |
|---|---|---|
| Paste clipboard images as PNG | ⚪ | Frontend/desktop concern. |
| Paste without Shift strips formatting | ⚪ | Frontend concern. |
| Default deck (current deck vs. by note type) | ❌ | No "last used deck/note type" persistence; deck selection is explicit when adding. |

### Browsing

| Anki feature | Status | Notes |
|---|---|---|
| Default search text | ⚪ | Frontend concern (could seed from a user pref later). |
| Ignore accents in search (slower) | ❌ | Search is exact; no accent-folding. |

## Syncing

| Anki feature | Status | Notes |
|---|---|---|
| Synchronize audio and images too | 🟡 | Server is authoritative (no device sync layer yet); media sync not implemented. |
| Automatically sync on open/close | ⚪ | N/A — client-server model, not device sync. |
| Periodically sync media | ❌ | No media sync. |
| Force changes in one direction | ❌ | No sync conflict flow (offline sync is a future feature). |
| AnkiWeb account / log out | 🟡 | We use JWT + server-side token revocation (`revoked_tokens`) and a `POST /logout` endpoint. Not AnkiWeb. |
| Self-hosted sync server | ⚪ | Not applicable to the platform architecture. |

## Backups

| Anki feature | Status | Notes |
|---|---|---|
| Automatic backups | ❌ | No automatic backup job. A manual `platform.db.backup` may exist from development, but there is no scheduled/automatic backup mechanism. |

## Summary of the biggest gaps

1. **Timeboxing** — no timebox time-limit preference.
2. **Audio preferences** — no audio support, hence no playback/interrupt options.
3. **Editing conveniences** — no "default deck" behaviour, no accent-insensitive search.
4. **Sync/backup** — no device sync, media sync, conflict handling, or automatic backups (offline sync is a planned future feature).

## Key architectural differences vs. Anki

- **Only two persisted user preferences**: `learn_ahead_seconds` and `day_start_hour`. Both live in the `user_preferences` table (per-user, not per-deck). Everything else in Anki's preferences dialog is either a client-side concern or unimplemented.
- **Day boundary is UTC-anchored** — Anki's "next day starts at" is relative to the user's timezone; we store only an hour (`day_start_hour`) with no per-user timezone yet, so the boundary is computed against UTC.
- **No device sync layer** — Anki's sync preferences (media sync, one-directional sync, auto-sync) are tied to its profile/sync architecture, which the platform does not reproduce. Our model is a central server backed by JWT sessions; offline sync is a stated future feature.
- **Preferences are not yet user-editable in code** — the `user_preferences` columns have defaults and are read by `study.rs`, but there is currently no endpoint to read/update them (unlike deck options, which have full CRUD).
