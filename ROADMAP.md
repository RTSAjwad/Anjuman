# ROADMAP.md

The ordered, single source of truth for what's next. Work these **top to
bottom**: each task is a prerequisite (or strong dependency) for the ones
below it.

Legend:

- `[ ]` not started
- `[~]` in progress
- `[x]` done

---

## 1. Migrate SQLite → Postgres   `[ ]`

The server currently uses SQLite (via `sqlx` with the `sqlite` feature) and an
embedded `platform.db` file. Migrate to Postgres.

**Approach (settled):** clean consolidated schema — **one migration** that
creates the full schema, plus a **second migration** that inserts seed data.
The ~34 existing `migrations/*.sql` files (all dev-stage, no production data)
are discarded, not converted one-for-one.

### Sub-tasks

- [ ] Add `postgres` (and its `runtime-tokio-rustls` / `migrate` features) to
  `server/Cargo.toml`; drop the `sqlite` feature. Decide on the exact sqlx
  feature set (e.g. add `uuid`/`chrono`/`json` if the Postgres types need them).
- [ ] Rewrite `server/src/db.rs`:
  - [ ] `SqlitePool` → `PgPool`, `Sqlite` → `Postgres`.
  - [ ] Remove the `PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON`
        bootstrap (Postgres manages these natively).
  - [ ] Reconsider `begin_immediate`: Postgres has no `BEGIN IMMEDIATE`;
        either drop it, use `BEGIN`, or use `BEGIN ISOLATION LEVEL SERIALIZABLE`
        where atomicity requires it. Audit every call site (there are several —
        `notes.rs`, `decks.rs`, `admin_users.rs`, `deck_options_handler.rs`,
        `note_types_handler.rs`, etc.).
- [ ] Write the consolidated schema migration (single `.sql`):
  - [ ] Convert `INTEGER PRIMARY KEY` → `BIGSERIAL PRIMARY KEY` (or
        `BIGINT GENERATED ALWAYS AS IDENTITY`).
  - [ ] Convert `INTEGER NOT NULL DEFAULT 0` booleans → `BOOLEAN`.
  - [ ] Convert `TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP` → `TIMESTAMPTZ`
        (decide: `NOW()` default) — audit `created_at`/`joined_at` semantics.
  - [ ] Convert `TEXT` CHECK-constrained enums (`role`, `state`) to
        `CREATE TYPE ... AS ENUM` or keep as `TEXT` + `CHECK` (decide; enum
        types are more idiomatic Postgres but affect `sqlx` mapping).
  - [ ] Preserve all foreign keys + `ON DELETE CASCADE` + indexes. Note
        `idx_cards_note_template` is `UNIQUE`, `deck_options` has
        `UNIQUE(school_id, name)`, etc.
  - [ ] Re-express the recursive CTEs (deck subtree) in Postgres syntax.
  - [ ] Fold in the *entire* evolved schema — do not miss the columns added by
        the many incremental SQLite migrations (deck `options_id`/`parent_id`,
        `student_card_states` fields, `reviews.state_before`, card
        `suspend`/`bury`/`flag`/`step_index`, `buried_at`/`bury_reason`,
        `user_preferences`, `deck_option_steps`, note-type/template refactor,
        `revoked_tokens`, user `first_name`/`last_name`). Cross-check against
        every `query!` in `server/src/handlers/*.rs`.
  - [ ] JSON columns: `notes.fields_json` is `TEXT` + manual serde today —
        decide whether to switch to Postgres `JSONB` (recommended) with `sqlx`
        `Json<T>` mapping.
- [ ] Write the seed-data migration (idempotent `INSERT ... ON CONFLICT DO
      NOTHING`): the system school (id 0), global default deck-options preset
      (id 0), the two built-in note types ("Basic", "Basic (and reversed)").
      Mirror what `20260728160017_seed_data.up.sql` and
      `20260822010000_builtin_note_types.up.sql` currently do.
- [ ] Convert the ~7 handler files that use SQLite-isms:
  - [ ] `unixepoch()` → `(EXTRACT(EPOCH FROM NOW()))::bigint` (or pass the
        timestamp from Rust via `chrono`/`std::time`).
  - [ ] `?` placeholders → `$1, $2, …` (note: some handlers use `$1`/`$2`
        already, e.g. the card-browser dynamic queries).
  - [ ] `last_insert_rowid()` → `RETURNING id`.
  - [ ] `INSERT OR IGNORE` → `INSERT ... ON CONFLICT DO NOTHING`.
  - [ ] Integer-boolean reads (`archived != 0`, `bury_new != 0`, `suspended`)
        → native `bool`.
  - [ ] The `Option<i64>` vs `i64` PRIMARY KEY quirk (SQLite) disappears —
        simplify the `.expect("... is NOT NULL")` sites.
- [ ] Update `flake.nix` dev shell: add a Postgres server (or a `postgresql`
      + a documented `DATABASE_URL` default, e.g.
      `DATABASE_URL=postgres://localhost/anjuman`). Update
      `server/.env.example` to a Postgres URL. Remove the `sqlite`/`sqlite3` if
      no longer used (keep if useful for inspecting stray `.db` files during
      transition).
- [ ] Delete `server/platform.db`, `platform.db.backup`, `platform.db.bak`
      (gitignored anyway, but clean them locally); remove the `*.db` ignore if
      truly no longer needed (keep `.env` ignore).
- [ ] `cargo build` + `cargo test` pass; `cargo sqlx prepare` (if you adopt
      offline query checking) or a live `cargo run` against Postgres succeeds;
      OpenAPI route coverage unchanged.

**Definition of done:** server boots against Postgres, all existing endpoints
work, migrations run cleanly from scratch (drop + recreate), and the SQLite
footprint (feature flag, `platform.db`, sqlite-isms) is gone from the repo.

---

## 2. Deck options — feature-complete with Anki   `[ ]`

Close the gaps in `server/DECK_OPTIONS_SUPPORT.md` as far as practical. The
matrix's "Summary of the biggest gaps" is the working list:

- [ ] **Display order** (largest gap) — implement `next_due_card`'s configurable
      gather/sort/review-order (deck, deck+random, ascending/descending/random;
      new/review mix order; interday ordering). Currently a single hardcoded
      order.
- [ ] **FSRS parameter optimization** — store per-school/user FSRS weights and
      add an optimizer endpoint (`compute_parameters`); today it's
      `FSRS::default()` weights only.
- [ ] **Lapses** — minimum interval after relearning; leech tracking
      (threshold + tag); support "empty relearning steps = 1 day".
- [ ] **Subdeck limit aggregation** — "selected deck governs the total" across
      a subtree.
- [ ] **Daily-limit fine controls** — "new cards ignore review limit", "limits
      start from top", per-deck "today only" overrides.
- [ ] **Hard-button step behaviour** — exact "average of first two steps" /
      "1.5× single step" computation.
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

## Notes

- **Ordering rationale:** task 1 is invisible-but-risky infrastructure (no API
  change) done *before* the feature work so that tasks 2 & 3 don't have to be
  migrated twice. Tasks 2/3 change the API, which task 4 then consumes.
- Update this file's checkboxes (`[ ]`→`[x]`) and status markers at the start
  and end of every sub-task, with a one-line note of what changed.
