# TIMESTAMPTZ migration plan (Phase 4 — timestamp sub-plan)

Full end-to-end `TIMESTAMPTZ` migration. This document is the authoritative
plan for converting the timestamp model from `i64` epoch-seconds to
`chrono::DateTime<Utc>`, preserving **identical scheduling behaviour**.

> Companion to [`docs/adr/postgres-migration.md`](./postgres-migration.md) (which covers Phases 0–3 and the
> non-timestamp SQL conversions). This doc is the *timestamp-specific* blast
> radius and safety rules.

---

## 1. Goal & the "identical behaviour" contract

- Store every timestamp as `TIMESTAMPTZ`.
- Represent every timestamp in Rust as `chrono::DateTime<Utc>`.
- Serialize timestamps over the wire as **RFC3339 strings** (`2026-09-27T19:56:34Z`).
- **The FSRS algorithm, interval lengths, "due now" classification, and
  day-boundary behaviour must be identical to the current whole-second
  `i64` implementation.**

### The single safety rule that guarantees identical behaviour

> **Capture `now` exactly once per operation as a `DateTime<Utc>`, and perform
> all interval/elapsed/day-boundary arithmetic in integer *seconds* derived
> from that one instant.**

Why: the current code truncates to whole seconds via `as_secs()`. If we let
`TIMESTAMPTZ`'s sub-second precision leak into the comparisons (e.g. mixing
`Utc::now()` in Rust with `NOW()` in SQL, or comparing a `due_at` with sub-second
precision against a separately-captured `now`), we introduce drifts that the
`i64` model never had. To preserve identical behaviour:

- Do **not** use SQL `NOW()` for scheduling decisions; always pass `now` from Rust.
- Derive `due_at`, `reviewed_at`, `last_reviewed_at` from the single captured `now`
  plus integer-second offsets.
- Keep day-boundary math as integer arithmetic on epoch-seconds
  (`ts.timestamp()`), then re-wrap in `DateTime<Utc>`.

---

## 2. Complete inventory of timestamp-bearing columns

| Column | Table | Type (now) | Used as |
|---|---|---|---|
| `created_at` | schools, users, classes, decks, notes, cards, note_types, deck_options | `TIMESTAMPTZ` (already) | audit/display |
| `joined_at` | class_members | `TIMESTAMPTZ` | display |
| `added_at` | deck_classes | `TIMESTAMPTZ` | audit |
| `shared_at` | deck_collaborators | `TIMESTAMPTZ` | audit |
| `due_at` | student_card_states | `TIMESTAMPTZ` | **scheduling** (due-now) |
| `last_reviewed_at` | student_card_states | `TIMESTAMPTZ` | **scheduling** (elapsed) |
| `buried_at` | student_card_states | `TIMESTAMPTZ` | **scheduling** (day-rollover) |
| `reviewed_at` | reviews | `TIMESTAMPTZ` | **scheduling** (daily limits) |
| `expires_at` | revoked_tokens | `TIMESTAMPTZ` | auth cleanup |
| `revoked_at` | revoked_tokens | `TIMESTAMPTZ` | audit |

> Good news: the schema (`0001_create_schema.sql`) is already `TIMESTAMPTZ` for
> all of these. So Phase 4's timestamp work is **entirely Rust + DTO changes**;
> no schema changes needed.

---

## 3. Contract (wire) fields to convert: `i64`/`String` → `DateTime<Utc>`

Add `chrono = { version = "0.4", features = ["serde"] }` to `contracts/Cargo.toml`
and `utoipa` already has `chrono` (openapi feature). Convert:

| File | Field | From → To |
|---|---|---|
| `study.rs` | `StudyCard.due_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `study.rs` | `StudyCard.buried_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `reviews.rs` | `ReviewResponse.due_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `reviews.rs` | `ReviewedCardState.due_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `cards.rs` | `CardModResponse.due_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `cards.rs` | `CardModResponse.buried_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `cards.rs` | `NoteModResponse.buried_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `cards.rs` | `RescheduleBody.due_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `cards.rs` | `CardBrowserResponse.due_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `cards.rs` | `CardBrowserResponse.buried_at` | `Option<i64>` → `Option<DateTime<Utc>>` |
| `cards.rs` | `CardBrowserResponse.created_at` | `String` → `DateTime<Utc>` |
| `classes.rs` | `ClassResponse.created_at` | `String` → `DateTime<Utc>` |
| `classes.rs` | `MemberResponse.joined_at` | `i64` → `DateTime<Utc>` |
| `decks.rs` | `DeckResponse.created_at` | `String` → `DateTime<Utc>` |
| `decks.rs` | `CollaboratorResponse.shared_at` | `i64` → `DateTime<Utc>` |
| `notes.rs` | `NoteResponse.created_at` | `String` → `DateTime<Utc>` |
| `users.rs` | `UserDetail.created_at` | `String` → `DateTime<Utc>` |
| `analytics.rs` | `DailyPoint.date` | `String` (date-only) → keep `String` (it's a `YYYY-MM-DD` label, NOT a timestamp) |
| `analytics.rs` | `StudentStatsWithEmail.last_active` | `Option<String>` → `Option<DateTime<Utc>>` |
| `analytics.rs` | `ClassCard.last_activity` | `Option<String>` → `Option<DateTime<Utc>>` |
| `analytics.rs` | `AttentionStudent.last_active` | `Option<String>` → `Option<DateTime<Utc>>` |

> Note: `DailyPoint.date` stays `String` — it's a rendered `%Y-%m-%d` label, not
> an instant. `applied_interval_secs`, `reps`, `lapses`, `flag`, `step_index`,
> `stability`, `difficulty` stay numeric (they're durations/counts, not instants).

This is a **wire-format change**: these fields move from UNIX integers / opaque
strings to RFC3339 strings. The generated OpenAPI (`/api-docs`) changes from
`type: integer` / `type: string` to `type: string, format: date-time`.

---

## 4. Server (non-DTO) changes: helper + arithmetic conversion

### 4.1 Replace epoch helpers with `chrono`

- `now_secs()` (in `study.rs`, `card_mod.rs`, `analytics.rs`, `dashboard.rs`,
  `reviews.rs` inline) → `Utc::now()` returning `DateTime<Utc>`.
- `day_start_utc(now: i64, hour: i64)` → rewrite to operate on `DateTime<Utc>`:
  compute `now.date_naive()` + `hour`, but **preserve integer second semantics**:
  ```
  let ts = now.timestamp();               // i64 whole seconds (identical to today)
  let seconds_past_midnight = ts % 86400;
  // ... exactly the existing modulo logic, then
  DateTime::<Utc>::from_timestamp(dst_ts, 0)
  ```
  This keeps the day-boundary *bit-identical* to today's `i64` math.

### 4.2 `reviews.rs` `apply_review` (the FSRS authority)

- `now` becomes a single `DateTime<Utc>` captured once.
- `elapsed_days` unchanged: `((now - last_reviewed).num_days().max(0))`. Since
  `last_reviewed_at` is now `DateTime<Utc>`, `now - last_reviewed` gives a
  `Duration`; `.num_days()` matches the old `(delta / 86400)` integer division
  (both floor to days). **Identical.**
- `due_at = now + interval_fsrs_secs` → `now + Duration::seconds(interval_fsrs_secs)`.
  `interval_fsrs_secs` stays `i64` (the FSRS/step math is untouched).
- `due_at`/`last_reviewed_at` set as `DateTime<Utc>` in the UPDATE.
- `applied_interval_secs = due_at - now` → `due_at.signed_duration_since(now).num_seconds()`.

### 4.3 `study.rs` (counts + next-card + predict_intervals)

- `deck_counts_for_student` / `next_due_card` SQL: replace `unixepoch()` with the
  bound `now` parameter; `due_at <= $n`, `due_at <= $n + ($m * INTERVAL '1 second')`.
  Keep comparisons on `TIMESTAMPTZ` columns against the bound `DateTime<Utc>`
  (sqlx passes it through faithfully).
- `predict_intervals`: `now` becomes `DateTime<Utc>`; the
  `(now - due).max(0) / 86400` → `(now - due).num_days()` (integer day).
- `CardRow.due_at`/`buried_at` become `Option<DateTime<Utc>>`.

### 4.4 `card_mod.rs`, `analytics.rs`, `dashboard.rs`, `classes.rs`, CRUD handlers

- `reschedule` (`RescheduleBody.due_at` now `DateTime<Utc>`): bind directly.
- `fetch_state`/`fetch_note_result_state` return `DateTime<Utc>` for
  `buried_at`/`due_at`.
- `start_of_today`/format helpers in `dashboard.rs` (`format_date`) stay on
  `timestamp()` for `%Y-%m-%d` rendering.

### 4.5 `jwt.rs` (auth)

- `revoked_tokens.expires_at`/`revoked_at` `i64` → `DateTime<Utc>`:
  - `Claims.iat`/`exp` stay `usize` epoch-seconds (JWT standard) — only the DB
    columns change.
  - `revoke_token(expires_at: i64)` → `DateTime<Utc>`; convert `claims.exp` via
    `Utc.timestamp_opt(exp, 0)`.
  - The cleanup job in `main.rs` (`DELETE ... WHERE expires_at <= ?`) →
    `expires_at <= $1` with a `DateTime<Utc>` bound.

---

## 5. SQL conversion (overlaps with main Phase 4, but timestamp-specific)

- `unixepoch()` in `study.rs` (`deck_counts_for_student`, `next_due_card`) →
  bound `DateTime<Utc>` params (`due_at <= $n`, `due_at <= $n + ($m * INTERVAL '1 second')`).
- `unixepoch()` in INSERT `created_at` (`users.rs`, `classes.rs`, `decks.rs`,
  `notes.rs`, `note_types_handler.rs`) → bind `Utc::now()`.
- `last_insert_rowid()` → `RETURNING id` (unchanged from main plan).
- Integer booleans → `bool` (unchanged from main plan; note `suspended`/`flag`:
  `suspended` → `bool`, `flag` stays `i64`).

---

## 6. Order of execution (safe, reviewable increments)

1. **Contract types first** — add `chrono` to `contracts`, convert the DTO
   timestamp fields. This is self-contained (contracts has no SQL).
2. **Schema already done** (Phases 2–3).
3. **`jwt.rs` + `main.rs` cleanup** (isolated auth timestamps).
4. **`study.rs` + `reviews.rs`** (the scheduler) — the highest-risk, do last and
   with a focused diff. Add a regression test for `day_start_utc` and interval
   math if feasible.
5. **`card_mod.rs`, `analytics.rs`, `dashboard.rs`, CRUD handlers** — mechanical.
6. **Resolve `query!` compile errors** via a live Postgres (already running) and
   regenerate `.sqlx` offline metadata (decision 6).
7. **Regenerate OpenAPI** and confirm route coverage (schemas change date formats).

---

## 7. Risks & mitigations

- **Day-boundary drift** (highest): mitigated by keeping the exact integer
  `% 86400` logic in `day_start_utc`, operating on `now.timestamp()`.
- **Sub-second `now` inconsistency**: mitigated by "capture `now` once" rule and
  never using SQL `NOW()` for scheduling.
- **Wire-format break**: intended and accepted; OpenAPI regenerated; client
  (future) reads RFC3339.
- **`elapsed_days` floor semantics**: `.num_days()` matches the old `/ 86400`
  integer division for positive deltas; negative deltas clamp to 0 identically.

---

## 8. Verification checklist

- [ ] `cargo build` green (server + contracts).
- [ ] Migrations apply to an empty DB.
- [ ] Study flow: GET/POST advance returns correct `due_at` (RFC3339) and
      `applied_interval_secs` unchanged.
- [ ] Daily-limit clamping and burial day-rollover behave as before (manual
      smoke test + comparison with a known-good trace).
- [ ] OpenAPI regenerated; 71 operations; date fields `format: date-time`.
