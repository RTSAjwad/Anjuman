# Anki Deck Options — Support Matrix

This document compares the deck options described in the [Anki manual](https://docs.ankiweb.net/deck-options.html) against what Anki Classroom supports. It also captures places where we intentionally diverge from Anki.

Legend:

- ✅ supported
- 🟡 partial / differs
- ❌ not supported
- ⚪ not applicable (FSRS-only platform — no SM-2 legacy algorithm)

## Presets

| Anki feature | Status | Notes |
|---|---|---|
| Presets shared across decks | ✅ | `deck_options` table + `decks.options_id`. Matches Anki's "dconf" preset model. |
| New decks use "Default" preset | ✅ | Falls back to global default preset (system school, id 0). |
| Add preset / Clone / Rename / Delete | ✅ | `POST`/`PATCH`/`DELETE /deck-options`. No dedicated "clone" endpoint — clone by creating a new preset from an existing one's values. |
| "Save to all subdecks" | ❌ | No bulk "assign preset to subtree" operation. |
| Options not retroactive | ✅ | Settings only persist; already-scheduled cards keep their existing `due_at`/state (same as Anki). |

## Subdecks

| Anki feature | Status | Notes |
|---|---|---|
| Subdecks can have their own preset | ✅ | Each deck has its own `options_id`; nested decks are supported. |
| Per-deck new/review limits vs. selected-deck total | 🟡 | Per-deck limits are supported. Anki's "selected deck governs the total" aggregation across a subtree during one session is **not** implemented — limits apply to the specific deck being studied. |
| Display order taken from selected deck | ❌ | No configurable display order (see Display Order). |

## Daily Limits

| Anki feature | Status | Notes |
|---|---|---|
| New cards/day | ✅ | `new_per_day` (default 20); counted via `state_before = 'new'` reviews per day. |
| Max reviews/day | ✅ | `review_per_day` (default 200). |
| Per-deck daily limits (preset / this deck / today only) | 🟡 | Per-deck limits via presets are supported; no "today only" override, no per-deck override independent of the preset. |
| New cards ignore review limit | ❌ | Not implemented. New cards are independently capped by `new_per_day` only. |
| Limits start from top (parent limits apply to subdecks) | ❌ | Not implemented. |

## New Cards — Learning Steps & Day Boundaries

| Anki feature | Status | Notes |
|---|---|---|
| Learning steps (e.g. `1m 10m`) | ✅ | Stored normalized in `deck_option_steps`; parsed from Anki-style `"1m 10m"` strings. |
| Hard button step behaviour (avg of first two steps; 1.5× single step) | 🟡 | Some Hard handling exists, but the exact "average of first two steps" / "1.5× single step" computation is **not** implemented — Hard advances via simplified logic. |
| Learn-ahead (show learning cards early) | ✅ | `learn_ahead_seconds` user pref, default 1200s (20 min). Matches Anki default. |
| Day boundaries (steps crossing a day boundary converted to days) | ✅ | `day_start_hour` user pref (default 4 AM); intraday vs interday learning computed in SQL. |

## New Cards — Graduation, Easy, Insertion

| Anki feature | Status | Notes |
|---|---|---|
| Graduating interval | ⚪ | FSRS computes graduation interval from memory state, not a fixed day count. |
| Easy interval (fixed) | ⚪ | FSRS-driven; Easy graduates using the FSRS interval. |
| Insertion order (sequential vs random) | ❌ | Not implemented. New cards use due/position ordering only. |

## Lapses

| Anki feature | Status | Notes |
|---|---|---|
| Relearning steps (e.g. `10m`) | ✅ | `deck_option_steps` relearning; default `10m`. |
| Blank steps → 1-day interval | 🟡 | We always default to `10m`; no support for "empty steps = 1 day". |
| Minimum interval (days after relearning) | ❌ | Not implemented. |
| Leeches | ❌ | Not implemented (no leech tracking/threshold/tagging). |

## Display Order

| Anki feature | Status |
|---|---|
| New card gather order (deck / deck+random / ascending / descending / random) | ❌ |
| New card sort order (card type / gathered / random…) | ❌ |
| New/review order (mix / before / after) | ❌ |
| Interday learning/review order | ❌ |
| Review sort order (due/random, relative overdueness, FSRS ascending retrievability, etc.) | 🟡 Due cards are sorted by a fixed priority (due date / state gathering order). No configurable review sort order, and specifically **no "ascending retrievability"**. |

> This is our largest gap in content ordering: `next_due_card` uses a single hardcoded gathering order.

## Burying

| Anki feature | Status | Notes |
|---|---|---|
| Bury new siblings | ✅ | `bury_new`. |
| Bury review siblings | ✅ | `bury_review`. |
| Bury interday learning siblings | ✅ | `bury_interday`. |
| Directional burying (earlier card types can't be buried by later) | ✅ | Implemented per gathering order. |
| Defaults | 🟡 | Anki documents these ON; we default **OFF** (a deliberate divergence). |

## Audio / Timers / Auto Advance / Easy Days

| Anki feature | Status |
|---|---|
| Audio auto-play toggle / skip question on replay | ❌ (no audio support yet) |
| Internal timer (max answer seconds) | 🟡 We record `response_time_ms`; no configurable 60s cap. |
| On-screen timer options | ❌ |
| Auto advance (show question/answer seconds) | ❌ |
| Easy Days (reduce workload on certain weekdays) | ❌ |

## FSRS

| Anki feature | Status | Notes |
|---|---|---|
| FSRS algorithm itself | ✅ | `fsrs` 6.6 crate, `FSRS::default()`. |
| FSRS enable/disable (global) | ⚪ | FSRS-only platform — always on, no SM-2 toggle. |
| Desired retention | ✅ | `desired_retention` (default 0.9), preset-scoped. |
| Desired retention per deck within a preset | ❌ | Retention is preset-level only. |
| FSRS parameter optimization | 🟡 | No stored weight columns; scheduling uses `FSRS::default()` weights. No optimizer (`compute_parameters`) endpoint yet. Significant gap — no parameter tuning. |
| Reschedule cards on change | ❌ | Changes only affect future reviews; no reschedule-on-change. |
| Minimum recommended retention / Health check / Simulator | ❌ | Removed or experimental in Anki; not implemented. |
| Learning/relearning steps < 1d guidance | ✅ | Steps supported; no hard block on ≥1d steps (like Anki, guidance only). |

## Advanced

| Anki feature | Status |
|---|---|
| Maximum interval | ❌ (FSRS intervals unbounded) |
| Historical retention | ❌ |
| Ignore cards reviewed before | ❌ |
| Starting ease | ⚪ (SM-2 concept; FSRS uses difficulty) |
| Easy bonus | ⚪ (SM-2) |
| Interval modifier | ⚪ (SM-2) |
| Hard interval | ⚪ (SM-2) |
| New interval | ⚪ (SM-2) |
| Custom scheduling (JS) | ❌ |

## Summary of the biggest gaps

1. **Display order** — none of Anki's gather/sort/review-order options; a single hardcoded order. The single largest missing feature area.
2. **FSRS parameter optimization** — `FSRS::default()` weights with no stored per-user/school parameters and no optimizer endpoint. (`desired_retention` *is* supported.)
3. **Lapses** — no minimum interval, no leech detection, no "empty steps = 1 day".
4. **Subdeck limit aggregation** — per-deck limits only; no "selected deck total" semantics.
5. **Daily-limit fine controls** — no "new cards ignore review limit", no "limits start from top", no "today only".
6. **Comfort/UX options** — audio, timers, auto-advance, easy days all absent.

## Key architectural differences vs. Anki

- **FSRS-only**: no SM-2 legacy algorithm, so SM-2-only options (starting ease, easy bonus, interval modifier, hard interval, new interval) are conceptually absent rather than "unsupported".
- **Steps are normalized** into `deck_option_steps` (one row per step) rather than stored as a space-separated string — enabling the SQL-side intraday/interday computation.
- **Day boundary is UTC-anchored** via a per-user `day_start_hour`, with no per-user timezone yet (Anki stores a full "next day starts at" timestamp per collection).
- **Burying defaults are OFF**, diverging from Anki's documented ON default.
