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

### Scope: server vs. client behaviour

Deck options fall into two kinds, and the distinction matters for how (and when)
we implement them:

- **Server-side behaviour** — the option changes how the server *schedules or
  orders* cards (steps, limits, leeches, burying, gather/sort/mix/review-sort,
  FSRS, easy days, new-card insertion). These are implemented end-to-end in the
  server, with `server/tests/` covering the behaviour.
- **Selection-only (client-side behaviour)** — the server's only job is to
  *store and return* the selection in the preset (e.g. on-screen timer, audio,
  auto-advance). There is **no server behaviour** to implement; the effect is a
  client concern (stage 4). When we add one of these now, we persist the option
  and test only its **round-trip through CRUD** (that the selection survives
  create/update/read) — the client behaviour is explicitly out of scope until
  the client stage, and is flagged in the matrix with “behaviour client-side”.

Each row below states which kind it is where it is not already obvious from a
✅/❌.

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
| New cards/day | ✅ | `new_per_day` (default 20); counted via `state_before = 'new'` reviews per day. In-app it is a **number, min 0, max 9999**; we store an unwrapped `i64` (no 0–9999 clamp — see divergence note). |
| Max reviews/day | ✅ | `review_per_day` (default 200). In-app **min 0, max 9999**; unwrapped `i64` (no clamp). Interday learning shares this limit (gathered first) — matches the in-app text. |
| Per-deck daily limits (preset / this deck / today only) | ✅ | Per-deck `preset`/`this_deck`/`today_only` override stored on `decks` (US-2.6), with `today_only` lazy expiry at the study-day boundary. |
| New cards ignore review limit | ⏳ | Deferred to after stage 7 — "collection-wide" in Anki; our school/user split makes its scope a stage-7 decision. |
| Limits start from top (parent limits apply to subdecks) | ⏳ | Deferred to after stage 7 (same rationale as above). |

## New Cards — Learning Steps & Day Boundaries

| Anki feature | Status | Notes |
|---|---|---|
| Learning steps (e.g. `1m 10m`) | ✅ | Stored normalized in `deck_option_steps`; parsed from Anki-style `"1m 10m"` strings. Units `s`/`m`/`h`/`d` (and a bare number = minutes) are supported (`parse_steps`). Default `1m 10m` matches the in-app "1 minute / 10 minutes" defaults. |
| Hard button step behaviour (avg of first two steps; 1.5× single step) | ✅ | Implemented in `apply_review` + `predict_intervals` via `hard_step_delay` (first step → avg of first two; single step → 1.5× capped at +1 day; other steps → repeat current). |
| Learn-ahead (show learning cards early) | ✅ | `learn_ahead_seconds` user pref, default 1200s (20 min). Matches Anki default. |
| Day boundaries (steps crossing a day boundary converted to days) | ✅ | `day_start_hour` user pref (default 4 AM); intraday vs interday learning computed in SQL. |

## New Cards — Graduation, Easy, Insertion

| Anki feature | Status | Notes |
|---|---|---|
| Graduating interval | ⚪ | FSRS computes graduation interval from memory state, not a fixed day count. |
| Easy interval (fixed) | ⚪ | FSRS-driven; Easy graduates using the FSRS interval. |
| Insertion order (sequential vs random) | ✅ | `insertion_order` enum on `deck_options` (default `sequential`). Sequential = monotonic `card_position_seq`. Random assigns a shuffled `position` on insert (`sync_card_rows`) and re-sorts the preset's existing new cards retroactively when the option changes. **Divergence (documented):** Anki's new-card position is a *global* `due` namespace; we scope the re-sort to the preset's decks (multi-tenant safety). Manual repositioning remains a separate deferred story. |

## Lapses

| Anki feature | Status | Notes |
|---|---|---|
| Relearning steps (e.g. `10m`) | ✅ | `deck_option_steps` relearning; default `10m`. Same unit logic as learning steps (`s`/`m`/`h`/`d` via `parse_steps`). |
| Empty relearning steps → skip relearning, FSRS recomputes interval | ✅ | Under FSRS, empty relearning steps skip the relearning phase and recompute the interval directly (US-2.4). The manual's "1 day" wording is the SM-2 rule; FSRS uses the FSRS interval. |
| Empty learning steps → FSRS controls short-term scheduling | ❌ | Not implemented (US-2.5). Matches Anki's "experimental" flag; deferred behind the non-experimental gaps. |
| Minimum interval | ⚪ | **SM-2 only** — the in-app FSRS Lapses section shows only relearning/leech options (no Minimum interval), confirming it is hidden under FSRS. Out of scope. |
| Leech threshold | ✅ | `leech_threshold` (default 8); counted on review-card "Again" only. In-app it is a **number, min 1, max 9999**; we store an unwrapped `i64`. |
| Leech action (Tag Only / Suspend Card) | 🟡 | `leech_action` enum. `SuspendCard` suspends at threshold; `TagOnly` is a documented no-op, and neither action tags the note (no tag system yet — see ROADMAP stage 6). `notes.leech_tagged_at` captures the leech marker. The in-app text confirms both actions *tag* the note; only the tag half is deferred. |

## Display Order

| Anki feature | Status |
|---|---|
| New card gather order (deck / deck-then-random-notes / ascending / descending / random notes / random cards) | ✅ US-2.8 (`new_gather_order`); random uses a deterministic per-student-day seed. |
| New card sort order (card type / gathered / card-type+random / random note+card type / random) | ✅ US-2.9 (`new_sort_order`); card-type ordering uses `note_type_templates.ord`, random uses the per-student-day seed. |
| New/review order (mix / before / after) | ✅ US-2.10 (`new_review_order`, default `mix`). `mix` uses Anki's stateless `Intersperser` (ratio-based even distribution — see the mix note below). `before`/`after` are fixed block orderings. |
| Interday learning/review order (mix / before / after) | ✅ US-2.11 (`interday_order`, default `mix`). Interday learning is always *gathered* first (shares the review limit); the option controls its display rank, with `mix` using the same `Intersperser`. |
| Review sort order (due-then-random, due-then-deck, deck-then-due, ascending/descending intervals, easy/difficult first, ascending/descending retrievability, relative overdueness, random, order added, latest added first) | ✅ US-2.12 (`review_sort_order`). All 13 in-app options supported; `interval` ↦ FSRS `stability`, `easy`/`difficult` ↦ FSRS `difficulty`. Two documented inconsistencies below. |

> The five selectors above are implemented across US-2.8–US-2.12.

### In-app selections vs. this implementation (consistency table)

Anki's web manual and the in-app selector sometimes disagree. This table maps
each in-app selection to what we implemented and flags where the two diverge.

| Selector | In-app selection | Implemented as | App↔manual consistency |
|---|---|---|---|
| New card gather order | Deck | `Deck` (subdecks in order, `c.position ASC`) | ✅ matches |
| | Deck, then random notes | `DeckThenRandomNotes` (subdeck order, random note within) | ✅ matches |
| | Ascending position | `Ascending` (`c.position ASC`) | ✅ matches |
| | Descending position | `Descending` (`c.position DESC`) | ✅ matches |
| | Random notes | `RandomNotes` (`md5(seed‖note_id)`) | ✅ matches (seed divergence is architectural, not a naming issue) |
| | Random cards | `RandomCards` (`md5(seed‖id)`) | ✅ matches |
| New card sort order | Card type then order gathered | `CardTypeThenGathered` (`tpl.ord ASC, c.position ASC`) | ✅ matches |
| | Order gathered | `Gathered` (preserves gather order) | ✅ matches |
| | Card type, then random | `CardTypeThenRandom` (`tpl.ord ASC, md5(…)`) | ✅ matches |
| | Random note, then card type | `RandomNoteThenCardType` (`md5(note)‖tpl.ord`) | ✅ matches |
| | Random | `Random` (`md5(seed‖id)`) | ✅ matches |
| New/review order | Mix with reviews | `Mix` (stateless `Intersperser`) | ✅ matches |
| | Show after reviews | `After` | ✅ matches |
| | Show before reviews | `Before` | ✅ matches |
| Interday learning/review order | Mix with reviews | `Mix` (stateless `Intersperser`) | ✅ matches |
| | Show after reviews | `After` | ✅ matches |
| | Show before reviews | `Before` | ✅ matches |
| Review sort order | (all 13) | 13 `ReviewSortOrder` variants | ⚠️ two mappings differ from the manual — see the two review-sort bullets in "Key architectural differences" |

## Burying

| Anki feature | Status | Notes |
|---|---|---|
| Bury new siblings | ✅ | `bury_new` (boolean, default off — matches the in-app default). |
| Bury review siblings | ✅ | `bury_review` (boolean, default off). |
| Bury interday learning siblings | ✅ | `bury_interday` (boolean, default off). |
| Directional burying (earlier card types can't be buried by later) | ✅ | Implemented per gathering order (intraday never buried; a later-priority answered card cannot bury an earlier-priority sibling).
| Defaults | ✅ | All three booleans are **off by default**, matching the in-app defaults. (The web manual's text describing burying ON is stale; the application itself defaults them off — so this is a manual-vs-app inconsistency, not a divergence of ours.) |

## Audio

| Anki feature | Status | Kind | Notes |
|---|---|---|---|
| Don't play audio automatically | ✅ | selection-only / client | Boolean. Persisted in `deck_options.dont_play_audio_automatically`; playback is a client behaviour (stage 4). No server behaviour. |
| Skip question when replaying answer | ✅ | selection-only / client | Boolean. Persisted in `deck_options.skip_question_when_replaying_answer`; purely client behaviour (stage 4). |

## Timers

| Anki feature | Status | Kind | Notes |
|---|---|---|---|
| Maximum answer seconds | ✅ | server | In-app it is a **number (min 1, max 7200, default 60)**. The client measures elapsed time and sends `response_time_ms`; `apply_review` clamps it to `deck_options.maximum_answer_seconds` at write time (validated `1..=7200`). The on-screen timer is a separate client option (US-2.14). |
| Show on-screen timer | ✅ | selection-only / client | Boolean, default off. Persisted in `deck_options.show_on_screen_timer`; counts time per card on the Study screen (client only, stage 4). |
| Stop on-screen timer on answer | ✅ | selection-only / client | Boolean, default off; "doesn't affect statistics". Persisted in `deck_options.stop_timer_on_answer` (client only, stage 4). |

**Client contract (stage 4):** the client runs the stopwatch and sends
`response_time_ms` on `StudyAdvanceBody`; it mirrors the `1..=7200` range in its
input UI, and *may* pre-clamp the on-screen value to `maximum_answer_seconds` for
an accurate display — but the server's clamp is authoritative, so pre-clamping is
optional polish, not a correctness requirement.

## Auto Advance

| Anki feature | Status | Kind | Notes |
|---|---|---|---|
| Seconds to show question for | ✅ | selection-only / client | F64 (1 dp), min 0.0, max 9999.0, default 0.0 (`0` disables). Persisted; client behaviour (stage 4). |
| Seconds to show answer for | ✅ | selection-only / client | F64 (1 dp), min 0.0, max 9999.0, default 0.0 (`0` disables). Persisted; client behaviour (stage 4). |
| Wait for audio | ✅ | selection-only / client | Boolean, default on. Persisted (`auto_advance_wait_for_audio`); client behaviour (stage 4). |
| Question action | ✅ | selection-only / client | Enum `show_answer` \| `show_card`, default `show_answer`. Persisted (`auto_advance_question_action`); client behaviour (stage 4). |
| Answer action | ✅ | selection-only / client | Enum `bury_card` \| `answer_again` \| `answer_good` \| `answer_hard` \| `show_reminder`, default `bury_card`. Persisted (`auto_advance_answer_action`); client behaviour (stage 4). |

**Decision** — the whole Auto Advance group is treated as selection-only / client: the server persists the values (seconds, wait-for-audio flag, and the two action enums) but performs no timing or advancing (stage 4).

## Easy Days

| Anki feature | Status | Kind | Notes |
|---|---|---|---|
| Easy Days (reduce workload on certain weekdays) | 🟡 | selection-only / client | One slider **per weekday** (`Minimum` \| `Reduced` \| `Normal`) stored as `easy_days` (7-elem array, Monday-first). **Persist-only**: Anki's Easy Days is wired into its *interval load balancer* (per-preset day projections over the ≤90-day fuzz window), which we do not implement, so the scheduling effect is a documented divergence deferred to a future "interval load balancer" story. |

## FSRS

| Anki feature | Status | Notes |
|---|---|---|
| FSRS algorithm itself | ✅ | `fsrs` 6.6 crate, `FSRS::default()`. |
| FSRS enable/disable (global) | ⚪ | A single collection-wide boolean toggle in Anki, shared by all presets. Anjuman is FSRS-only, so we omit it — always on, no SM-2 toggle. |
| Desired retention | ✅ | `desired_retention` (default 0.9), a **70–99% slider** in-app. We store a `f64` validated `0.0..=1.0` (no 70–99 clamp — see divergence note). |
| Desired retention per deck (deck scoping) | 🟡 | The in-app retention selector has a **Preset / This deck** scope toggle (same as the daily limits); our `desired_retention` is preset-scoped only — per-deck retention is not implemented. See stage-7 per-deck personalisation. |
| FSRS parameters | 🟡 | Stored per-preset as a JSONB weight vector (`fsrs_parameters`); empty = `FSRS::default()` and `apply_review` consumes them via `FSRS::new`. The **optimizer** that *produces* weights (the in-app parameters editor, search filter `preset: "Default" ~is:suspended`, and "Optimise Current/All Presets") is still missing (US-2.18b). `historical_retention` is also deferred with the optimizer. |
| Reschedule cards on change | ⏳ | Collection-wide, **not saved** (a transient action), in-app. Descoped to after stage 7 alongside the other collection-wide toggles. |
| Check health when optimizing | ⏳ | Collection-wide boolean (default off), in-app; only performed for "Optimise Current Preset". Descoped to after stage 7 (collection-wide). |
| FSRS Simulator (Experimental) | ⏳ | The "FSRS Simulator (Experimental)" button and the "Help Me Decide (Experimental)" button open **two different simulators** (they display different graphs). Both are descoped to a **future post-client stage** (simulation is a UI-heavy feature; see ROADMAP Notes). (`Compute minimum recommended retention` was **removed upstream** in Anki 25.07.) |
| Learning/relearning steps < 1d guidance | ✅ | Steps supported; no hard block on ≥1d steps (like Anki, guidance only). |

## Advanced

> The in-app Advanced section (under FSRS) shows only the FSRS-relevant options
> below; the SM-2-only options are hidden under FSRS (we mark them ⚪).

| Anki feature | Status | Notes |
|---|---|---|
| Maximum interval | ✅ | **Number, default 36500 (≈100 years), min 0, max 36500.** Caps the review interval; at the cap Hard/Good/Easy give the same delay. `apply_review` clamps the FSRS interval to `deck_options.maximum_interval` (stored `stability` stays uncapped). We validate `1..=36500` (reject `0`): Anki's scheduler floors the max at 1 (`maximum.max(1)`), so `0` is redundant with `1` — a documented UI-only divergence. |
| Historical retention | ❌ | **Percentage, default 90%, min 50%, max 100%.** FSRS-only: fills gaps in missing review history. Not implemented. |
| Ignore cards reviewed before | ❌ | **Date field, default 01/01/1970 (epoch).** Cards reviewed before this date are ignored when optimizing FSRS parameters. Not implemented (depends on the optimizer). |
| Custom scheduling (JS) | ⏳ | Text-area JS hook, **collection-wide**, "use at your own risk". Descoped (collection-wide) — see stage-7 note. |
| Starting ease | ⚪ | SM-2 only (hidden under FSRS); FSRS uses difficulty. |
| Easy bonus | ⚪ | SM-2 only. |
| Interval modifier | ⚪ | SM-2 only. |
| Hard interval | ⚪ | SM-2 only. |
| New interval | ⚪ | SM-2 only. |

## Summary of the biggest gaps

1. **Display order** — now complete for gather/sort/review-order (US-2.8–US-2.12); see the architectural divergences below.
2. **FSRS parameter optimization** — `FSRS::default()` weights with no stored per-user/school parameters and no optimizer endpoint. (`desired_retention` *is* supported.)
3. **Subdeck limit aggregation** — per-deck limits only; no "selected deck total" semantics.
4. **Daily-limit fine controls** — no "new cards ignore review limit", no "limits start from top", no "today only".
5. **Comfort/UX options** — audio, on-screen timer, and auto-advance are
   selection-only (client-side, no server behaviour); "Maximum answer seconds"
   is a small server-side gap (cap `response_time_ms`), and Easy Days is a
   server-side scheduling gap.

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
- **Burying defaults are OFF — and this *matches* the app.** The web manual's
  prose describes burying as on by default, but the in-app options are
  **off by default**, and we follow the app. This is a manual-vs-app
  inconsistency, not a divergence of ours.
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
  (intraday learning → interday learning → review → new).
- **"Mix with reviews" is Anki's `Intersperser`, implemented statelessly.**
  Anki's `mix` is `Intersperser`
  (`rslib/src/scheduler/queue/builder/intersperser.rs`): a pure ratio-based even
  distribution of two already-sorted queues — `ratio = (a_len+1)/(b_len+1)`, and
  draw from `b` when `(b_idx+1)*ratio < (a_idx+1)`. It is **not** a due-date
  merge and needs **no** materialized queue, so we implement it directly from
  `seen_today` + the due-now class counts (`intersperse_draw_b` in `study.rs`),
  with `Mix` restored as the default (matching Anki). The three-bucket *count*
  split (interday learning shares the `review` limit but is separately counted)
  is captured via a persisted `reviews.interday` flag — see ROADMAP Notes.
- **Review sort names "Ascending/Descending intervals" (not "ease") map to FSRS
  `stability`.** The in-app FSRS list names options 4–5 "Ascending intervals" /
  "Descending intervals", and 6–7 "Easy cards first" / "Difficult cards first" —
  the manual's SM-2-era "Ascending/Descending ease" wording does not even appear
  under FSRS. We map "intervals" to FSRS `stability`, and "easy/difficult" to FSRS
  `difficulty` ASC/DESC (SM-2 ease has no FSRS analogue).
- **"Relative overdueness" appears in-app despite the manual marking it removed
  under FSRS.** The in-app FSRS review-sort selector lists all 13 options
  including "relative overdueness"; the web manual describes it as removed in
  favour of retrievability. We implement it (`(now - due)/stability DESC`, most
  overdue first) to match the in-app list. Note it and retrievability are
  monotonic transforms of the same underlying overdue-ratio, so "relative
  overdueness" and "descending retrievability" (and their ascending counterparts)
  produce the *same* ordering — overdueness is Anki's linear proxy for the exact
  FSRS forgetting-curve retrievability.
- **Desired retention is a 70–99% range in-app, but we accept any `0.0..=1.0`.**
  Anki's UI constrains desired retention to a [70%, 99%] slider (the manual says
  "you can set your desired retention below 0.7 with the expert edits to the
  config, but it is not recommended"); we store a `f64` and validate only the
  `0.0..=1.0` envelope, without the 70–99 clamp. Recorded as a deliberate
  relaxation — revisit if we ever mirror Anki's slider in the client, at which
  point the client (or server validation) should re-impose the range.
- **Daily-limit numbers are 0–9999 in-app, but we store an unwrapped `i64`.**
  `new_per_day` and `review_per_day` are `min 0, max 9999` in Anki; we validate
  only non-negativity (no 9999 cap), same as the retention relaxation. Revisit
  if the client mirrors Anki's numeric input bounds.

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
