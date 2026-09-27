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

**Approach (settled):** clean consolidated schema — **one** migration that
creates the full schema, plus a **second** migration that inserts seed data.
The ~34 existing `migrations/*.sql` files (all dev-stage, no production data)
are discarded, not converted one-for-one.

> **Detailed plan:** [`docs/postgres-migration.md`](./docs/postgres-migration.md)
> — phased: (0) Postgres runtime → (1) deps + `db.rs` → (2) consolidated schema
> → (3) seed → (4) handler SQL conversion → (5) flake/OpenAPI/cleanup. Read it
> before starting; it enumerates every SQLite-ism by file and the open decisions
> to lock first.

### Sub-tasks (summary)

- [ ] **Phase 0** — decide + implement the Nix-idiomatic Postgres runtime
      (services-flake / process-compose / NixOS module / Docker); set
      `DATABASE_URL`; update `.env.example`.
- [ ] **Phase 1** — sqlx `postgres` feature (drop `sqlite`); rewrite `db.rs`
      (`PgPool`, drop `PRAGMA`, replace `BEGIN IMMEDIATE`); rename
      `SqlitePool`→`PgPool` across `state.rs`, `auth/jwt.rs`, `deck_options.rs`,
      `note_types.rs`, and handler helpers.
- [ ] **Phase 2** — write `0001_schema.sql` (full consolidated schema; see plan
      §Phase 2 for the column-type mapping and the 17-table checklist).
- [ ] **Phase 3** — write `0002_seed.sql` (system school, default preset,
      built-in note types, dev school/users/class/deck/notes).
- [ ] **Phase 4** — convert handler SQL: `?`→`$n`, `unixepoch()`→epoch-now,
      `last_insert_rowid()`→`RETURNING id`, `INSERT OR IGNORE`→`ON CONFLICT`,
      int-booleans→`bool` (with SQL casts where the wire stays `i64`),
      `Option<i64>` PK guards removed.
- [ ] **Phase 5** — flake Postgres provisioning, `.env.example`/`.gitignore`
      cleanup, delete old migrations + `platform.db*`, confirm OpenAPI coverage
      unchanged, update ROADMAP/AGENTS.

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
