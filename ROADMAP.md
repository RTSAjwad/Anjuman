# ROADMAP.md

The ordered, single source of truth for what's next. Work these **top to
bottom**: each task is a prerequisite (or strong dependency) for the ones
below it.

Legend:

- `[ ]` not started
- `[~]` in progress
- `[x]` done

---

## 1. Migrate SQLite → Postgres   `[x]`

Done. The server runs on Postgres (TIMESTAMPTZ, JSONB, ENUM types, BOOLEAN)
via a consolidated `0001_create_schema.sql` + `0002_seed_data.sql`, provisioned
by `services-flake` (`nix run .#anjuman`). All handler SQL + the full
`DateTime<Utc>` timestamp migration landed; `cargo build`/`test` pass and
OpenAPI route coverage is unchanged (71 operations).

> Detailed plans: `docs/postgres-migration.md`, `docs/timestamptz-migration.md`.

### Sub-tasks (summary)

- [x] **Phase 0** — Nix-idiomatic Postgres runtime (`flake-parts` +
      `services-flake`; `nix run .#anjuman`).
- [x] **Phase 1** — sqlx `postgres` feature; `db.rs` `PgPool`; type renames.
- [x] **Phase 2** — `0001_create_schema.sql` consolidated schema.
- [x] **Phase 3** — `0002_seed_data.sql`.
- [x] **Phase 4** — handler SQL conversion + full `TIMESTAMPTZ`/
      `JSONB`/`ENUM`/`bool` migration.
- [x] **Phase 5** — flake/docs/cleanup; OpenAPI coverage unchanged.

**Definition of done:** ✅ server boots against Postgres, all 71 endpoints
work, migrations run cleanly from empty, SQLite footprint removed.

---

## 2. Deck options — feature-complete with Anki   `[~]`

Close the gaps in `server/DECK_OPTIONS_SUPPORT.md` as far as practical. The
matrix's "Summary of the biggest gaps" is the working list:

- [ ] **Display order** (largest gap) — Decomposed into US-2.8–US-2.12 (gather,
      sort, new/review, interday, review-sort). See below.
- [ ] **FSRS parameter optimization** — store per-school/user FSRS weights and
      add an optimizer endpoint (`compute_parameters`); today it's
      `FSRS::default()` weights only.
- [x] **Lapses** — leech threshold/action + empty relearning steps. Done
      (US-2.2–US-2.4). Minimum interval is SM-2 — out of scope.
- [x] **US-2.2 — Leech threshold**

      **As** a student failing a review card repeatedly,
      **I want** the card to be flagged a leech after N review-card lapses,
      **so that** I can spot time-sink cards.

      **Acceptance criteria**
      - [x] `lapses` counts review-card "Again" (state Review) only, not
            learning/relearning "Again".
      - [x] Default leech threshold is 8.
      - [x] Threshold configurable via deck options (`leech_threshold`).
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Half-threshold re-warnings (no notification system).
- [x] **US-2.3 — Leech action (enum)**

      **As** a student,
      **I want** a leech card to be suspended (configurably),
      **so that** it stops consuming study time until I unsuspend it.

      **Acceptance criteria**
      - [x] `leech_action` is an enum: `TagOnly` | `SuspendCard`.
      - [x] `SuspendCard` suspends the card at threshold (default action).
      - [x] `TagOnly` is accepted on the wire but not yet effective.
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope / documented divergence**
      - Tagging the note (both actions tag in Anki) — blocked on a tag system
        (ROADMAP stage 6). `TagOnly` is a no-op until tags land.
- [x] **US-2.4 — Empty relearning steps**

      **As** a student who lets FSRS control short-term scheduling,
      **I want** an empty relearning-steps list to skip the relearning phase,
      **so that** FSRS recomputes the interval directly on a lapse.

      **Acceptance criteria**
      - [x] Empty relearning steps are accepted (no "at least one step" error).
      - [x] On `Again` from a review card, the card stays `review` and its
            interval is recomputed via FSRS (no relearning phase).
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Empty *learning* steps (see US-2.5).
- [ ] **US-2.5 — Empty learning steps → FSRS short-term scheduling**

      **As** a student who lets FSRS control short-term scheduling,
      **I want** an empty learning-steps list to let FSRS schedule the
      learning phase too,
      **so that** I can drop the fixed step ladder entirely.

      **Acceptance criteria**
      - TBD — needs an `fsrs`-crate investigation first: how does `fsrs 6.6`
        expose FSRS-5 short-term (learning-phase) intervals, and what does
        "Again" mean without a step ladder (can be ≥1 day).
      - [ ] New/learning cards schedule via FSRS rather than `learning_steps`.
      - [ ] Each criterion has a `server/tests/` test.

      **Matches Anki's "experimental" flag** — the manual marks
      "leaving the (re)learning steps field empty" as experimental; we mirror
      that status and defer this behind the non-experimental deck-options gaps.

      **Notes**
      - Requires revisiting whether short-term scheduling is a small branch or
        a real scheduler change (depends on the `fsrs` crate's learning-phase
        API). Place after Display order / parameter optimization / daily limits.
- [ ] **Subdeck limit aggregation** — "selected deck governs the total" across
      a subtree.
- [ ] **Daily-limit fine controls** — implemented as US-2.6 below (per-deck
      `preset`/`this_deck`/`today_only` override on `new_per_day`/`review_per_day`).
- [ ] **`new_cards_ignore_review_limit` + `limits_start_from_top`** — descoped
      until after stage 7. These are "collection-wide" in Anki; our multi-tenant
      school/user split makes their scope a stage-7 decision (see stage 7 design
      question #6).
- [x] **US-2.6 — Per-deck daily-limit overrides**

      **As** a teacher tailoring one deck,
      **I want** a per-deck `preset`/`this deck`/`today only` override on the
      new-cards and review limits,
      **so that** one deck can have its own caps without forking a whole preset.

      **Acceptance criteria**
      - [x] A deck in `preset` mode uses its shared preset's limit (unchanged).
      - [x] `this_deck` overrides the limit for that deck only; sibling decks
            sharing the preset are unaffected.
      - [x] `today_only` applies today and falls back to the preset limit on the
            next study day (lazy expiry).
      - [x] New and review limits are independently selectable (per-limit mode).
      - [x] `this_deck`/`today_only` without a value → `400`.
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Subdeck limit aggregation (US-2.7).
      - Stage-7 per-user personalisation (precedence reconciled later).

      **Model**
      - Columns on `decks` (school-authored, teacher/admin): `new_per_day_mode`
        + `review_per_day_mode` (`ENUM preset/this_deck/today_only`, default
        `preset`), nullable `new_per_day_override`/`review_per_day_override`, and
        `new_per_day_today_date`/`review_per_day_today_date` for lazy expiry.
        `deck_options.new_per_day`/`review_per_day` remain the preset base.
- [x] **US-2.7 — Subdeck limit aggregation**

      **As** a student studying a deck with subdecks,
      **I want** each subdeck's own limit to cap gathering from that subdeck,
      while the selected deck's limit caps the session total,
      **so that** limits compose correctly across the deck tree.

      **Acceptance criteria**
      - [x] Gathering honours each subdeck's effective limit (per-subdeck cap).
      - [x] The selected deck's limit caps the overall session total.
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - `limits_start_from_top` (collection-wide; deferred to after stage 7).
        Base aggregation is algorithm-neutral and built now; the toggle is the
        deferred piece.

      **Relation to US-2.6** — resolves each subdeck's *effective* limit via the
      same `effective_daily_limits` helper.
- [x] **US-2.1 — Hard-button step behaviour** — implement Anki's exact
      learning/relearning Hard rules. See `PLANNING.md` §2.

      **As** a student reviewing a (re)learning card,
      **I want** the Hard button to follow Anki's step rules,
      **so that** my scheduling matches Anki's expected intervals.

      **Acceptance criteria**
      - [x] On the first learning step, Hard = average of the first two steps.
      - [x] With a single step, Hard = 1.5× that step (capped at +1 day).
      - [x] On any other step, Hard repeats the current step.
      - [x] `predict_intervals` mirrors `apply_review` (frontend hints agree).
      - [x] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Review (graduated) cards: Hard already uses the FSRS interval.
- [ ] **US-2.8a — Add `cards.position` (prereq for display order)**

      **As** the scheduler,
      **I want** a per-card position column,
      **so that** "ascending/descending position" and several sort orders are
      meaningful.

      **Acceptance criteria**
      - [ ] `cards.position BIGINT NOT NULL` exists, defaulting to creation
            order (backfill = `id`-monotonic) so existing data is ordered.
      - [ ] New cards get a monotonic position on insert.
      - [ ] Each criterion has a `server/tests/` test (or a migration-verified
            backfill).

      **Out of scope**
      - Manual reordering UI (separate future story).
- [ ] **US-2.8 — New card gather order**

      **As** a student,
      **I want** to choose how new cards are gathered (deck / ascending /
      descending / random notes / random cards),
      **so that** I control which new cards are prioritised.

      **Acceptance criteria**
      - [ ] `new_gather_order` enum on `deck_options` (default `deck`).
      - [ ] `deck` gathers subdecks in order, each in ascending position.
      - [ ] `ascending`/`descending` order by `cards.position`.
      - [ ] `random_notes`/`random_cards` use a deterministic per-student-day
            seed (see divergence note).
      - [ ] Each criterion has a `server/tests/` test.

      **Out of scope**
      - "Deck, then random notes" (in-app variant) — treated as `deck` subset.
- [ ] **US-2.9 — New card sort order**

      **As** a student,
      **I want** to choose how gathered new cards are sorted (card type /
      gathered / card-type+random / random note+card type / random),
      **so that** sibling cards are spaced the way I prefer.

      **Acceptance criteria**
      - [ ] `new_sort_order` enum on `deck_options` (default card-type order).
      - [ ] Card-type ordering uses `note_type_templates.ord`.
      - [ ] Deterministic per-student-day seed for random variants.
      - [ ] Each criterion has a `server/tests/` test.
- [ ] **US-2.10 — New/review order**

      **As** a student,
      **I want** to choose whether new cards mix with, precede, or follow
      review cards,
      **so that** I control the study session shape.

      **Acceptance criteria**
      - [ ] `new_review_order` enum (mix / before / after), default mix.
      - [ ] `before`/`after` reorder the gathering class priority.
      - [ ] Each criterion has a `server/tests/` test.
- [ ] **US-2.11 — Interday learning/review order**

      **As** a student,
      **I want** to choose whether interday (re)learning cards mix with,
      precede, or follow review cards,
      **so that** I can front-load or defer harder cards.

      **Acceptance criteria**
      - [ ] `interday_order` enum (mix / before / after), default mix.
      - [ ] Interday learning is always *gathered* first (limit applied first),
            but its *display* order vs review follows the setting.
      - [ ] Each criterion has a `server/tests/` test.
- [ ] **US-2.12 — Review sort order**

      **As** a student,
      **I want** to choose the review sort order (due/random, due/deck,
      deck/due, ascending/descending interval, ascending/descending ease,
      ascending retrievability),
      **so that** I can clear a backlog or prioritise due cards sensibly.

      **Acceptance criteria**
      - [ ] `review_sort_order` enum (default due-date-then-random).
      - [ ] **Ascending retrievability** (FSRS `R`) is the FSRS sort; SM-2
            "relative overdueness" is ⚪ out of scope.
      - [ ] Each criterion has a `server/tests/` test.

      **Out of scope**
      - SM-2 "relative overdueness" (FSRS uses ascending retrievability).
- [ ] Cross-check the remaining ❌/🟡 rows and either implement or consciously
      descope each (record the descope decision in the matrix).

Each item must keep the OpenAPI spec in sync (new/changed DTOs → regenerated
`/api-docs`).

---

## 3. Preferences — feature-complete with Anki   `[ ]`

Close the gaps in `server/PREFERENCES_SUPPORT.md`. The backend currently
persists only two prefs (`day_start_hour`, `learn_ahead_seconds`) and has
**no endpoint** to read/update them. The matrix's "Summary of the biggest
gaps" is the working list:

- [ ] **Add preferences CRUD** — `GET/PATCH /preferences` (per-user), using the
      existing `user_preferences` table (or a generalized key/value row).
- [ ] **Timebox time limit** — implement the missing timebox preference.
- [ ] **Audio-related preferences** — depends on adding audio support (media
      storage/serving); likely a larger cross-cutting item — decide scope.
- [ ] **Editing conveniences** — "default deck" persistence (last-used
      deck/note type); accent-insensitive search (or fold into a search-token
      improvement).
- [ ] Timezone correctness — the current day-boundary is UTC-anchored via
      `day_start_hour`; consider a per-user timezone column.
- [ ] Cross-check the remaining ⚪/❌ rows and record descope decisions in the
      matrix.

---

## 4. Frontend — implement the client   `[ ]`

Replace the counter-demo core with the real domain, consuming
`anjuman_contracts` and the server API.

- [ ] Add `anjuman_contracts` as a path dependency of the client `shared` crate
      (and decide whether `utoipa`'s `openapi` feature is needed client-side —
      it should **not** be).
- [ ] Model the client domain in `client/shared/src/app.rs`:
      - [ ] `Model` (auth state, decks, notes, cards, study session, classes).
      - [ ] `Event`, `ViewModel`, `Effect` (add `crux_http` + `crux_kv` +
            `crux_time` capabilities — the core is currently `Render`-only).
      - [ ] Consume `anjuman_contracts` DTOs for HTTP payloads.
- [ ] Implement the auth flow (login → store JWT → `/me`).
- [ ] Implement stable screens first: decks, notes, cards, study — then fold in
      deck-options/preferences UI once tasks 2 & 3 land.
- [ ] Keep the FFI `Bridge`/`codegen` surface working as the model grows;
      regenerate bindings.
- [ ] `cargo build`/`cargo test` + `trunk serve` succeed.

---

## 5. Backfill — user stories + tests for existing features   `[ ]`

After the feature work settles, harden the *existing* surface: characterise it
and lock it down with the story→test discipline (see `PLANNING.md`). This runs
last so feature stages can move fast without a large backfill up front, while
still ending with full regression coverage. New features added in stages 2–4
should still ship *their own* stories + tests inline, not wait for this stage.

- [ ] Inventory every endpoint/feature (the 71 operations + the scheduling
      core) and write a `US-5.<n>` story for each, with acceptance criteria —
      derived from behaviour, not the support matrices.
- [ ] Add a `server/tests/` test for each story's acceptance criteria (using
      the `TestApp` harness from task 1), pinning current behaviour.
- [ ] Add `CruxCore` `update` + `effects()` tests for the client core surface
      so it has full coverage alongside the server.
- [ ] Record any gaps/descope found while characterising (behaviour that is
      undefined or surprising) as follow-up stories, not silently.

**Definition of done:** every endpoint has a `US-5.<n>` story with a passing
`server/tests/` test; no orphan behaviour.

---

## 6. Tags — note/collection tagging   `[ ]`

A general tag system (note-level tags: add, remove, filter, browse/search by
tag). This is a cross-cutting feature in its own right, deferred until after the
core feature stages settle.

- [ ] `note_tags` table (or equivalent) + schema migration.
- [ ] CRUD + list/browse/search-by-tag endpoints on `anjuman_contracts`.
- [ ] Client (stage 4) consumes tags in the browser/search UI.
- [ ] **Revisit leeches** — wire the leech `TagOnly` action (currently a
      documented no-op, see US-2.3) to actually emit the `leech` tag, and back
      the `SuspendCard` action's "also tag the note" behaviour. Today leech
      le marks the note via `notes.leech_tagged_at` only.

---

## 7. Per-user deck options / personal scheduling   `[ ]`

Allow a student to personalise scheduling for decks they study without mutating
shared, school-scoped presets. Today deck options are school-scoped and
**only teacher/admin-authored** — a deliberate divergence from Anki (where deck
options belong to the single user). This stage makes the personalisation model
**layered overrides** rather than replacing shared presets.

### Why

- Anki deck options are per-user; Anjuman's are per-school. The school preset
  solves *consistency* (a teacher sets `1m 10m` for the whole class), but it
  locks every student of a shared deck to identical scheduling.
- A student adopting a shared deck should be able to set **their own** limits,
  steps, or retention for their study without mutating the teacher's preset.

### Design questions to settle (before implementing)

- [ ] **Override target** — do personal overrides attach to a *deck* or to a
      *preset*? (Per-deck personal options vs. per-user preset clones.)
- [ ] **Storage** — a `user_deck_options(user_id, deck_id, …)` table, per-user
      preset clones, or a delta-compare model?
- [ ] **Permission model** — a `decks.allow_personal_options BOOLEAN` so a
      teacher can choose per-deck whether students may override. Turns today's
      hard "school-only" rule into a per-deck choice.
- [ ] **Fork vs. inherit** — does a personal override fork a snapshot of the
      preset (deviating independently), or diff against the live preset?
- [ ] **Analytics/comparison** — how do diverging student options affect class
      analytics and cross-student comparison?
- [ ] **Interaction with daily limits** — personal "new-per-day" limits overlap
      with the per-user daily-limit prefs; resolve before finalising both to
      avoid two competing mechanisms.

### Definition of done

- Effective options resolve as *personal override if present, else school
  preset*, with an explicit per-deck permission to admit overrides.
- A student can study a shared deck with their own limits/steps without
  touching the teacher's preset.
- Documented in `DECK_OPTIONS_SUPPORT.md` "Key architectural differences".

---

## Notes

- **Ordering rationale:** task 1 is invisible-but-risky infrastructure (no API
  change) done *before* the feature work so that tasks 2 & 3 don't have to be
  migrated twice. Tasks 2 & 3 change the API, which task 4 then consumes.
  Task 5 (backfill) runs last so the existing surface gets full story+tests
  coverage *after* the feature work settles, rather than front-loading it.
  Task 6 (tags) is a deferred cross-cutting feature that revisits the leech
  `TagOnly` no-op from US-2.3. Task 7 (per-user deck options) makes scheduling
  personalisation layered (personal override → school preset) instead of
  Anki's strict single-user model.
- Update this file's checkboxes (`[ ]`→`[x]`) and status markers at the start
  and end of every sub-task, with a one-line note of what changed.
