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

- [ ] **Display order** (largest gap) — implement `next_due_card`'s configurable
      gather/sort/review-order (deck, deck+random, ascending/descending/random;
      new/review mix order; interday ordering). Currently a single hardcoded
      order.
- [ ] **FSRS parameter optimization** — store per-school/user FSRS weights and
      add an optimizer endpoint (`compute_parameters`); today it's
      `FSRS::default()` weights only.
- [ ] **Lapses** — leech threshold/action + empty relearning steps. See
      US-2.2–US-2.4 below. (Minimum interval is SM-2 — out of scope.)
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
- [ ] **US-2.4 — Empty relearning steps**

      **As** a student who lets FSRS control short-term scheduling,
      **I want** an empty relearning-steps list to skip the relearning phase,
      **so that** FSRS recomputes the interval directly on a lapse.

      **Acceptance criteria**
      - [ ] Empty relearning steps are accepted (no "at least one step" error).
      - [ ] On `Again` from a review card, the card stays `review` and its
            interval is recomputed via FSRS (no relearning phase).
      - [ ] Each criterion has a `server/tests/` test.

      **Out of scope**
      - Empty *learning* steps (the manual flags both as experimental; keep to
        relearning for now).
- [ ] **Subdeck limit aggregation** — "selected deck governs the total" across
      a subtree.
- [ ] **Daily-limit fine controls** — "new cards ignore review limit", "limits
      start from top", per-deck "today only" overrides.
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

## Notes

- **Ordering rationale:** task 1 is invisible-but-risky infrastructure (no API
  change) done *before* the feature work so that tasks 2 & 3 don't have to be
  migrated twice. Tasks 2 & 3 change the API, which task 4 then consumes.
  Task 5 (backfill) runs last so the existing surface gets full story+tests
  coverage *after* the feature work settles, rather than front-loading it.
  Task 6 (tags) is a deferred cross-cutting feature that revisits the leech
  `TagOnly` no-op from US-2.3.
- Update this file's checkboxes (`[ ]`→`[x]`) and status markers at the start
  and end of every sub-task, with a one-line note of what changed.
