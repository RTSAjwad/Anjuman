# Anki Deck Options — Support Matrix

This document compares the deck options described in the [Anki manual](https://docs.ankiweb.net/deck-options.html) against what Anki Classroom supports. It also captures where we intentionally diverge from Anki.

> **Policy: FSRS-only, no SM-2.** Anjuman supports the FSRS scheduler only. We
> do **not** implement SM-2-specific features; such options are marked ⚪ and are
> out of scope. Before implementing any option, first classify it SM-2 vs FSRS.

Legend:

- ✅ supported
- 🟡 partial / differs
- ❌ not supported yet
- ⚪ SM-2-only / not applicable (no SM-2 legacy algorithm)
- ⏳ deferred — scoping/decision outstanding (see referenced roadmap stage)
- 📝 planned — story written, not yet implemented

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
| Per-deck new/review limits vs. selected-deck total | ✅ | Subdeck aggregation (US-2.7): each subdeck's effective limit caps gathering from that subdeck, while the selected deck's limit caps the total. `limits_start_from_top` (the collection-wide toggle) is deferred to after stage 7. |
| Display order taken from selected deck | ❌ | No configurable display order (see Display Order). |

## Daily Limits

| Anki feature | Status | Notes |
|---|---|---|
| New cards/day | ✅ | `new_per_day` (default 20); counted via `state_before = 'new'` reviews per day. |
| Max reviews/day | ✅ | `review_per_day` (default 200). |
| Per-deck daily limits (preset / this deck / today only) | ✅ | Per-deck `preset`/`this_deck`/`today_only` override stored on `decks` (US-2.6), with `today_only` lazy expiry at the study-day boundary. |
| New cards ignore review limit | ⏳ | Deferred to after stage 7 — "collection-wide" in Anki; our school/user split makes its scope a stage-7 decision. |
| Limits start from top (parent limits apply to subdecks) | ⏳ | Deferred to after stage 7 (same rationale as above). |

## New Cards — Learning Steps & Day Boundaries

| Anki feature | Status | Notes |
|---|---|---|
| Learning steps (e.g. `1m 10m`) | ✅ | Stored normalized in `deck_option_steps`; parsed from Anki-style `"1m 10m"` strings. |
| Hard button step behaviour (avg of first two steps; 1.5× single step) | ✅ | Implemented in `apply_review` + `predict_intervals` via `hard_step_delay` (first step → avg of first two; single step → 1.5× capped at +1 day; other steps → repeat current). |
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
| Empty relearning steps → skip relearning, FSRS recomputes interval | ✅ | Under FSRS, empty relearning steps skip the relearning phase and recompute the interval directly (US-2.4). The manual's "1 day" wording is the SM-2 rule; FSRS uses the FSRS interval. |
| Empty learning steps → FSRS controls short-term scheduling | ❌ | Not implemented (US-2.5). Matches Anki's "experimental" flag; deferred behind the non-experimental gaps. |
| Minimum interval | ⚪ | **SM-2 only** (not shown under FSRS). Out of scope. |
| Leech threshold | ✅ | `leech_threshold` (default 8); counted on review-card "Again" only. |
| Leech action (Tag Only / Suspend Card) | 🟡 | `leech_action` enum. `SuspendCard` suspends at threshold; `TagOnly` is a documented no-op, and neither action tags the note (no tag system yet — see ROADMAP stage 6). `notes.leech_tagged_at` captures the leech marker. |

## Display Order

| Anki feature | Status |
|---|---|
| New card gather order (deck / deck-then-random-notes / ascending / descending / random notes / random cards) | ✅ US-2.8 (`new_gather_order`); random uses a deterministic per-student-day seed. |
| New card sort order (card type / gathered / card-type+random / random note+card type / random) | ✅ US-2.9 (`new_sort_order`); card-type ordering uses `note_type_templates.ord`, random uses the per-student-day seed. |
| New/review order (mix / before / after) | ✅ US-2.10 (`new_review_order`). `before` moves new ahead of review; `mix`/`after` keep reviews first (true interleave of `mix` is a stateless limitation — see the divergence note). |
| Interday learning/review order (mix / before / after) | ✅ US-2.11 (`interday_order`). Interday learning is always gathered first (limit applied first); the option controls its display rank vs review. |
| Review sort order (due-then-random, due-then-deck, deck-then-due, ascending/descending intervals, easy/difficult first, ascending/descending retrievability, relative overdueness, random, order added, latest added first) | ✅ US-2.12 (`review_sort_order`). All 13 in-app options supported; `interval` ↦ FSRS `stability`, `easy`/`difficult` ↦ FSRS `difficulty`. Two documented inconsistencies below. |

> The five selectors above are implemented across US-2.8–US-2.12.

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
| Minimum recommended retention / Health check / Simulator | ❌ | Not implemented. (`Compute minimum recommended retention` additionally was **removed upstream** in Anki 25.07.) |
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

1. **Display order** — now complete for gather/sort/review-order (US-2.8–US-2.12); see the architectural divergences below.
2. **FSRS parameter optimization** — `FSRS::default()` weights with no stored per-user/school parameters and no optimizer endpoint. (`desired_retention` *is* supported.)
3. **Subdeck limit aggregation** — per-deck limits only; no "selected deck total" semantics.
4. **Daily-limit fine controls** — no "new cards ignore review limit", no "limits start from top", no "today only".
5. **Comfort/UX options** — audio, timers, auto-advance, easy days all absent.

## Key architectural differences vs. Anki

- **Deck options are school-scoped, not per-user.** Anki deck options belong to
  the single user; Anjuman presets are school-scoped and teacher/admin-authored
  (shared so a class studies consistently). A student consuming a shared deck is
  currently locked to the school preset. See ROADMAP stage 7 (per-user deck
  options / personal scheduling), which plans a layered personal-override model
  rather than this current all-or-nothing rigidity.
- **FSRS-only: no SM-2.** Anjuman supports the FSRS scheduler only and
  deliberately does **not** implement SM-2-specific features. When the manual
  describes an option, we first classify it SM-2 vs FSRS; SM-2-only options
  (starting ease, easy bonus, interval modifier, hard interval, new interval,
  and minimum interval) are marked ⚪ below and are out of scope, not "missing".
- **Steps are normalized** into `deck_option_steps` (one row per step) rather than stored as a space-separated string — enabling the SQL-side intraday/interday computation.
- **Day boundary is UTC-anchored** via a per-user `day_start_hour`, with no per-user timezone yet (Anki stores a full "next day starts at" timestamp per collection).
- **Burying defaults are OFF**, diverging from Anki's documented ON default.
- **Hard-button delay precision**: Anki's manual says the first-step Hard delay for
  `1m 10m` is "6m", but the raw average is `5m30s`; the real client shows `<6m`.
  We store the exact average in seconds (`330s`) rather than replicating Anki's
  minute-rounded *display*. The internal value matches Anki; only the rendered
  label differs.
- **Random display-order seed is per-student-day, not per-session.** Anki seeds
  its random gather/sort orders per *study session*; our study flow is stateless
  (the client loops `POST /decks/:id/study` with no server session), so we seed
  deterministically from `student_id + day_start`. The queue is stable within a
  day and varies across days — a deliberate divergence from Anki's per-session
  seeding.
- **New-card ordering composes as *gather order primary, sort order secondary*.**
  Anki's model is two phases: gather (select the candidate set + a coarse order)
  then sort (re-order the gathered set — the *final* display order). Our
  single-card-per-request scheduler approximates this as gather-then-sort keys in
  one `ORDER BY`, so the sort's final ordering isn't applied as a second, fully
  separate phase. For the default (`deck` + `card_type_then_gathered`) and most
  combinations the result matches; strictly matching Anki would require a
  two-phase gather→sort pipeline (see below).
- **Display-order pairings the manual leaves unspecified are resolved to Anki's
  gathering order.** The `new_review_order` and `interday_order` options each
  reorder one pair (new-vs-review, interday-vs-review), but the manual never
  fixes interday-vs-new. We resolve those gaps to the underlying gathering order
  (intraday learning → interday learning → review → new), and treat both `mix`
  modes as their gathered-first default (interday before review, review before
  new) — a documented approximation, since a true interleave needs a materialized
  queue (stateless limitation).
- **Review sort "ease" is surfaced via FSRS `difficulty`, not SM-2 ease.** Anki's
  "Easy cards first" / "Difficult cards first" sort on the SM-2 *ease* factor,
  which FSRS has no analogue for; we map them to FSRS `difficulty` ASC/DESC. The
  in-app label still says "easy"/"difficult", but the underlying key is FSRS
  difficulty — a documented FSRS-vs-SM-2 inconsistency.
- **"Relative overdueness" appears in-app despite the manual marking it removed
  under FSRS.** The in-app FSRS review-sort selector lists all 13 options
  including "relative overdueness"; the web manual describes it as removed in
  favour of retrievability. We implement it (`(now - due)/stability DESC`, most
  overdue first) to match the in-app list. Note it and retrievability are
  monotonic transforms of the same underlying overdue-ratio, so "relative
  overdueness" and "descending retrievability" (and their ascending counterparts)
  produce the *same* ordering — overdueness is Anki's linear proxy for the exact
  FSRS forgetting-curve retrievability.

## Open questions / under-documented Anki behaviour

Where Anki's manual is imprecise and we had to make a judgement call:

- **Hard-button "average of first two steps" rounding** — the manual states the
  rule but not the unit/rounding; its `6m` example is a display-rounded value
  (`5m30s`), confirmed by testing (`<6m`). We use the exact averaged seconds.
- **1.5× single-step "at most 1 day longer" cap** — we implement
  `min(1.5×step, step + 86400)` using round-to-nearest for the 1.5× factor. Anki
  does not specify whether it rounds or floors the fractional result. We chose
  round-to-nearest as the standard, unbiased default; revisit only if a concrete
  fractional-step case shows Anki diverging.
