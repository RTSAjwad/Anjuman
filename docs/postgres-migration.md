# Migration Plan — SQLite → Postgres

This is the detailed execution plan for ROADMAP task 1. It supersedes the
summary checklist in `ROADMAP.md`. Work it top-to-bottom; each phase is
independently verifiable.

## Goals & non-goals

- **Goal:** server boots against Postgres, all 71 endpoints behave correctly,
  migrations run cleanly from an empty database.
- **Non-goal:** no feature work, no data preservation (dev-only data).
- **Note:** one small, deliberate wire change is in scope: four `suspended`
  fields become `bool` (was `i64` 0/1) per the settled boolean decision. JSON
  field order/content is otherwise unchanged.
- **Approach (settled):** discard the ~34 SQLite migration files; write **one**
  schema migration + **one** seed migration for Postgres.

---

## Phase 0 — Decide the Postgres runtime, before any code

The flake must provision Postgres "in a Nix-flake-idiomatic way". Decide and
implement this *first*, because `cargo build` (`sqlx::query!` compile-time
checks) and `sqlx migrate run` both need a reachable database. Options (pick
one):

- **(A) NixOS module / `services.postgresql`** in the user's NixOS config (if
  the dev machine is NixOS) — the flake only documents `DATABASE_URL`.
- **(B) `services-flake` / `process-compose`** — a dev-shell that spins up a
  throwaway `postgres` on `nix develop` (idiomatic for flake-based dev).
- **(C) Docker Compose** — a `compose.yaml` with `postgres`, documented in the
  README (not "Nix-idiomatic", but simplest).

> Recommendation: **(B)** `services-flake` (or `process-compose`) so
> `nix develop` gives you a live local Postgres automatically, keeping the
> "everything via Nix" story. This is the one decision to lock **before** Phase 1.

**Deliverable:** a documented, reproducible local Postgres + a default
`DATABASE_URL=postgres://anjuman:anjuman@localhost:5432/anjuman` (or similar).
Update `server/.env.example` to the Postgres URL.

---

## Phase 1 — Dependencies & connection layer

1. `server/Cargo.toml` — sqlx features (final):
   - Drop `sqlite`.
   - Keep/add `runtime-tokio-rustls`, `migrate`, `macros`.
   - Add `postgres`, `chrono`, `json`, `uuid` (for `TIMESTAMPTZ`, `JSONB`,
     and identity columns as needed).
2. `server/src/db.rs`:
   - `use sqlx::postgres::{PgPool, PgPoolOptions}` + `PgPool`/`Postgres`.
   - `connect()` → `PgPoolOptions::new().max_connections(N).connect(DATABASE_URL)`.
   - **Delete** the `PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON` block
     (both are Postgres-native/always-on).
   - **`begin_immediate`** — Postgres has no `BEGIN IMMEDIATE`. Replace with a
     plain `pool.begin()` → `Transaction<'static, Postgres>`. SQLite's
     `SQLITE_BUSY` concern doesn't exist; plain `READ COMMITTED` is sufficient.
3. **Global type rename** — `SqlitePool` → `PgPool` and `Sqlite` → `Postgres`
   across these files (explicit type annotations, not just `db.rs`):
   - `src/state.rs` (`AppState.db`)
   - `src/auth/jwt.rs` (`verify_token`, `revoke_token`, `use sqlx::SqlitePool`)
   - `src/deck_options.rs` (`get_options`, `load_steps`, `options_for_deck`,
     `replace_steps`)
   - `src/note_types.rs` (`get_note_type`, `list_note_types`, etc.)
   - `src/handlers/{analytics,card_browser,card_mod,study}.rs` (helper fns)
   - Any remaining `sqlx::Sqlite` / `SqlitePool` references (`grep` to confirm zero).

**Verify:** `cargo build` succeeds (forces the type rename to be complete).

---

## Phase 2 — Write the consolidated schema migration (`0001_schema.sql`)

One `.sql` file (plus a `0002` seed). Standardize on idiomatic Postgres:

### Column-type mapping (apply uniformly)

> **Decisions locked (2026-09-27):** timestamps → `TIMESTAMPTZ`; JSON → `JSONB`;
> enum columns → Postgres `ENUM` types; boolean flags → `bool`; offline query
> checking via `cargo sqlx prepare` + committed `.sqlx/`.

| SQLite | Postgres (final) |
|---|---|
| `INTEGER PRIMARY KEY` | `BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY` |
| `INTEGER NOT NULL DEFAULT 0` (bool) | `BOOLEAN NOT NULL DEFAULT FALSE` |
| `INTEGER` epoch-seconds (`due_at`, `joined_at`, `reviewed_at`, `expires_at`, `buried_at`, `added_at`, `shared_at`, `revoked_at`, `created_at`) | **`TIMESTAMPTZ`** — implies a `chrono::DateTime<Utc>` (`sqlx` `chrono` feature) + `Utc.timestamp_opt(...)` → `.to_utc()`/`now()` rewrite across the codebase; `now_secs()`/`SystemTime` epoch-seconds helpers are replaced by `Utc::now()`/`chrono` arithmetic. |
| `TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP` (`created_at`) | `TIMESTAMPTZ NOT NULL DEFAULT NOW()` (consistent with the above). |
| `fields_json TEXT` / `field_names TEXT` (JSON) | **`JSONB`** (sqlx `json` feature; wrap with `sqlx::types::Json<...>` in handlers; `serde_json::to_string`/`from_str` round-trips become `Json`-bound). |
| CHECK-enum `TEXT` (`role`, `state`, `kind`) | **`CREATE TYPE ... AS ENUM`** (`user_role`, `membership_role`, `card_state`, `step_kind`) — implies `sqlx` maps them as `String` still, and `#[derive(sqlx::Type)]` or `String` bindings; enum types in Postgres are `TEXT`-compatible for `String` reads. |
| `REAL` (stability, difficulty, retention) | `DOUBLE PRECISION` (maps to `f64`). |

### Wire-contract impact (decision 5: booleans)

Switching `suspended`/`buried`/`archived` booleans to native `bool` **does change
four `anjuman_contracts` DTOs** (a deliberate, small wire change — previously
`i64` 0/1):

- `cards::CardModResponse.suspended`: `i64` → `bool`
- `cards::NoteModResponse.suspended`: `i64` → `bool`
- `cards::CardBrowserResponse.suspended`: `Option<i64>` → `Option<bool>`
- `study::StudyCard.suspended`: `i64` → `bool`

(`archived` already `bool`. The `flag` fields — `StudyCard.flag`,
`CardBrowserResponse.flag`, `FlagResponse.flag`, `SetFlag.flag` — are **0-7 flag
colors, not booleans**, and stay `i64`/`i32`.) This changes the generated OpenAPI
(integer → boolean) and must be reflected in `/api-docs` — an accepted,
intentional delta, not a regression.

### Full table set (cross-checked against every migration — no missed columns)

1. **`schools`** — id, name, created_at.
2. **`users`** — id, school_id, email (UNIQUE), password_hash, role (CHECK), first_name, last_name, created_at.
3. **`classes`** — id, school_id, name, description, archived (BOOL), created_by, created_at.
4. **`class_members`** — class_id, user_id, role (CHECK teacher/student), joined_at; PK (class_id, user_id).
5. **`decks`** — id, school_id, title, description, created_by, created_at, **parent_id** (self-FK cascade), **options_id** (FK → deck_options, SET NULL).
6. **`deck_classes`** — deck_id, class_id, added_at; PK (deck_id, class_id).
7. **`deck_collaborators`** — deck_id, user_id, shared_at; PK (deck_id, user_id).
8. **`note_types`** — id, school_id, name, field_names, sort_field (default ''), created_by, created_at; UNIQUE(school_id, name).
9. **`note_type_templates`** — **id** (identity), note_type_id, **ord**, name, front_pattern, back_pattern; UNIQUE(note_type_id, ord). *(Final schema uses `id` + `ord`, NOT `template_index`.)*
10. **`notes`** — id, note_type_id (FK), fields_json, created_at. *(No `deck_id` — cards own deck relationship.)*
11. **`cards`** — id, note_id, deck_id, **template_id** (FK cascade), created_at; UNIQUE(note_id, template_id).
12. **`student_card_states`** — student_id, card_id, state (CHECK), stability, difficulty, due_at, last_reviewed_at, reps, lapses, **flag** (default 0), **step_index** (default 0), **suspended** (BOOL default false), **buried_at**, **bury_reason**; PK (student_id, card_id). *(No `buried_until`.)*
13. **`reviews`** — id, student_id, card_id, rating (CHECK 1–4), reviewed_at, response_time_ms, **state_before** (default 'review').
14. **`deck_options`** — id, school_id, name, desired_retention, bury_new, bury_review, bury_interday, new_per_day, review_per_day, created_at; UNIQUE(school_id, name). *(No step columns — normalized out.)*
15. **`deck_option_steps`** — id, options_id (FK cascade), kind (CHECK), step_index, seconds; UNIQUE(options_id, kind, step_index).
16. **`user_preferences`** — user_id (PK, FK cascade), learn_ahead_seconds (default 1200), **day_start_hour** (default 4).
17. **`revoked_tokens`** — jti (PK), user_id, expires_at, revoked_at.

All FKs: `ON DELETE CASCADE` where SQLite had it; `ON DELETE SET NULL` for
`decks.options_id`; `note_types.created_by` nullable. Reproduce every `idx_*`
index and the unique indexes (`idx_cards_note_template`, `idx_decks_parent`,
`idx_due_cards`, …).

**Verify:** `sqlx migrate run` against a fresh `createdb` applies cleanly; the
recursive CTE syntax is valid Postgres (`WITH RECURSIVE` works).

---

## Phase 3 — Write the seed migration (`0002_seed.sql`)

Idempotent (`ON CONFLICT DO NOTHING`), merging the old seed migrations:

1. System school (`id=0`, `'System'`).
2. Global default deck-options preset (`id=0`, `school_id=0`, `'Default'`:
   retention 0.9, bury off, new 20 / review 200) + its 3 `deck_option_steps`.
3. Dev school (`id=1`, `'Springfield High'`).
4. Three dev users (teacher/admin/student, existing Argon2 hashes).
5. Dev class `'Biology 101'` + one membership.
6. Two built-in note types `'Basic'` (`id=1`), `'Basic (and reversed)'` (`id=2`) + templates.
7. Dev deck + `deck_classes` link.
8. The 10 biology notes + 10 cards.

Translate `INSERT OR IGNORE` → `INSERT ... ON CONFLICT DO NOTHING`; keep
explicit `id`s (needed for FKs and stable references).

**Verify:** `migrate run` from empty seeds exactly once; re-running doesn't duplicate.

---

## Phase 4 — Convert handler SQL (the SQLite-isms)

Grep-verified inventory; convert in this order:

1. **`?` → `$n` placeholders** in every query across all handlers + `jwt.rs` +
   `deck_options.rs` + `note_types.rs`. (Some files already use `$1`/`$2`.)
2. **`unixepoch()` → timestamp handling** (because of the `TIMESTAMPTZ`
   decision):
   - INSERT `created_at`/`reviewed_at`/etc.: bind `Utc::now()` (or a
     `chrono::DateTime<Utc>`) from Rust, not an epoch integer.
   - **comparisons** (scheduling logic in `study.rs`): the "due now" checks
     `due_at <= unixepoch()` become `due_at <= NOW()` (SQL-side, using Postgres
     `NOW()` returns `TIMESTAMPTZ`); `unixepoch() + ?` (learn-ahead) becomes a
     computed `NOW() + ($n * INTERVAL '1 second')`.
   - Files: `classes.rs`, `decks.rs` (×2), `notes.rs` (×2), `note_types_handler.rs`
     (×2), `users.rs`, `study.rs` (×3), plus the `now_secs()`/`SystemTime` helpers
     in `analytics.rs`/`card_mod.rs`/`dashboard.rs` → `Utc::now()`/`chrono`.
3. **`last_insert_rowid()` → `RETURNING id`** (`.fetch_one` → `row.id`):
   `users.rs`, `classes.rs`, `decks.rs` (create + duplicate chains), `notes.rs`,
   `note_types_handler.rs` (clone + create_template), `deck_options_handler.rs`.
   Inside transactions use `.fetch_one(&mut **tx)`. The duplicate_deck
   INSERT…SELECT flow is the trickiest (keep transaction structure).
4. **`INSERT OR IGNORE` → `ON CONFLICT DO NOTHING`**:
   `jwt.rs` (`jti`), `notes.rs` `sync_card_rows` (`(note_id, template_id)`).
5. **Integer-boolean reads → `bool`** (per decision 5): `archived`, `bury_new`,
   `suspended`, `bury_review`, `bury_interday` read as native `BOOLEAN` → `bool`
   (remove the `!= 0` sites). The four `suspended` wire fields become `bool` in
   `anjuman_contracts` (see §Phase 2 "Wire-contract impact"). The 0-7 `flag`
   fields stay integers.
6. **PRIMARY KEY `Option<i64>` → `i64`**: Postgres returns non-null; remove the
   `.expect("... is NOT NULL")` guards in `admin_users.rs`, `classes.rs`,
   `decks.rs`, `search_users.rs`, `note_types.rs`.

### Care points

- Recursive CTEs in `study.rs` + `decks.rs` (`WITH RECURSIVE subtree`) — `?`→`$1`;
  keep the `SUM(CASE...) "total!"` type annotations (`"...": i64` is SQLite
  syntax — Postgres uses `as "total!: i64"` from the alias, verify).
- `card_browser.rs` already uses `$1`/`$2` + dynamic SQL; confirm `NULLS LAST`,
  `LIMIT/OFFSET` params.

**Verify:** `cargo build` + `cargo test`; `cargo run` against Postgres and
smoke-test CRUD + study flow.

---

## Phase 5 — OpenAPI, flake, docs, cleanup

1. `flake.nix` — replace `sqlite`/`sqlite3` + the `platform.db` shellHook with
   Phase-0 Postgres provisioning; remove the `platform.db` init block.
2. `server/.env.example` → Postgres `DATABASE_URL`.
3. `server/.gitignore` — drop `platform.db*`; keep `target/`, `.env`.
4. Delete `server/platform.db{,.backup,.bak}` (local cleanup).
5. Delete the old `server/migrations/*.sql` (replaced by `0001`/`0002`).
6. OpenAPI — confirm route coverage unchanged (71 operations via `/api-docs`).
7. Update `ROADMAP.md` (flip task 1 to `[x]`) + `AGENTS.md` §5 database line.

---

## Risks / open decisions — ALL RESOLVED (2026-09-27)

1. **Postgres runtime** (Phase 0) → **in the flake** (`services-flake`/
   `process-compose` so `nix develop` brings up a local Postgres).
2. **Timestamp model** → **`TIMESTAMPTZ`** (Rust side: `chrono::DateTime<Utc>`,
   sqlx `chrono` feature). Broader rewrite than epoch-seconds — replaces
   `now_secs()`/`SystemTime`/`as_secs()` helpers and the `unixepoch()` SQL.
3. **`fields_json`/`field_names`** → **`JSONB`** (sqlx `json` feature +
   `sqlx::types::Json<T>` in handlers).
4. **Enum columns** → **Postgres `ENUM` types** (`user_role`, `membership_role`,
   `card_state`, `step_kind`).
5. **`suspended` wire type** → **`bool`** (four DTO fields change; `flag` stays
   0-7 integer).
6. **Offline query checking** → **`cargo sqlx prepare` + committed `.sqlx/`**
   metadata (so `cargo build` doesn't require a live DB), per recommendation.

---

## Suggested execution (for an agent)

`Phase 0` → `1` → `2` → `3` → `4` → `5`, committing after each phase with
`cargo build` green at every checkpoint. Phases 2–3 (SQL) and 4 (handlers) are
the high-volume mechanical parts and are the best candidates to split across
parallel agents with disjoint file scopes (migrations vs. handlers).
