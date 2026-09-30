# AGENTS.md

Guidance and authoritative references for working on this repository. Read
this first — it orients you in the monorepo and points at where the *why* and
*what's next* live.

---

## 1. What this repo is

**Anjuman** — an Anki-inspired spaced-repetition platform with organisation and
collaboration (classes, roles, shared decks). One git repository containing
three independent Cargo workspaces, linked by path dependencies:

| Folder | Crate | What it is |
|---|---|---|
| `contracts/` | `anjuman_contracts` | The **single source of truth** for wire DTOs (request/response + shared enums like `UserRole`). Consumed by both the server and (soon) the client. |
| `server/` | `anjuman_server` | The Axum backend (HTTP API, FSRS scheduling, Postgres). |
| `client/` | (workspace) `shared` + `shells/*` | The Crux cross-platform app (Leptos shell implemented; SwiftUI/WinUI/Compose planned). |

> Note: these are **separate Cargo workspaces**, not one top-level workspace.
> This is deliberate — the client needs a WASM/size-optimized release profile,
> the server a standard throughput profile, and separate `/target` and
> `Cargo.lock` per workspace. See `client/ARCHITECTURE.md` §2 and §4 for the
> boundary model. The server and client consume `anjuman_contracts` via
> `path = "../contracts"`.

## 2. What's next / current work

The ordered task list lives in [`ROADMAP.md`](./ROADMAP.md). Read it before
starting any new piece of work — it is the single source of truth for what is
in progress, what is next, and the concrete acceptance criteria for each task.
User stories and the acceptance-criteria→test discipline are defined in
[`PLANNING.md`](./PLANNING.md); read that before writing stories or tests.

## 3. Where the detailed docs live

- `PLANNING.md` — the user-story format, the acceptance-criteria→test mapping,
  and the ordering rules. The process doc for all feature work.
- **Anki manual** — the authoritative reference for feature behaviour, since
  Anjuman aims for maximum Anki compatibility: <https://docs.ankiweb.net/>.
  Feature-specific pages: [deck options](https://docs.ankiweb.net/deck-options.html),
  [preferences](https://docs.ankiweb.net/preferences.html). Read the relevant
  page before implementing a feature; the support matrices below are derived
  from (and link back to) it.
- `client/AGENTS.md` — client-specific version pinning and gotchas (Crux 0.20,
  Leptos 0.8, BoltFFI 0.30.1, thaw 0.5-beta). Read before touching `client/`.
- `client/ARCHITECTURE.md` — the client's design rationale (core-first, thin
  shells, FFI boundaries).
- `client/SCREENS_SUPPORT.md` — client screens + client-side behaviour matrix.
  Drives roadmap task 4 (the client analog of the two server support matrices).
- `server/DECK_OPTIONS_SUPPORT.md` — Anki deck-options feature matrix (what's
  supported vs. missing). Drives roadmap task 2.
- `server/PREFERENCES_SUPPORT.md` — Anki preferences feature matrix. Drives
  roadmap task 3.
- `contracts/src/lib.rs` — module map of the shared wire types.
- `server/src/openapi.rs` — the generated OpenAPI document (served at
  `/api-docs`; there is no hand-written `api.yaml` anymore).

## 4. Build / run / test

Everything goes through the root Nix dev shell (no global Rust on PATH):

```sh
nix develop                # full shell (client + server toolchains)
nix develop .#client       # client-only
nix develop .#server       # server-only

# contracts
cargo build --manifest-path contracts/Cargo.toml
cargo build --manifest-path contracts/Cargo.toml --features openapi

# server
cargo build --manifest-path server/Cargo.toml
cargo test  --manifest-path server/Cargo.toml

# client (inside its workspace)
cd client && cargo build && cargo test
cd client/shells/leptos && trunk serve
```

Server `cargo test` runs integration tests (`server/tests/`) against a real
Postgres. They use a **dedicated test database** (`ANJUMAN_TEST_DATABASE_URL`,
default `postgres://127.0.0.1:5432/anjuman_test`) that the harness migrates and
resets per test — so Postgres must be running (`nix run .#anjuman`) and the
test DB must exist (`createdb anjuman_test`). See `server/tests/common/mod.rs`.

## 5. Working conventions

- **FSRS-only, no SM-2.** Anjuman supports the FSRS scheduler only; do not
  implement SM-2-specific features. The Anki manual interleaves SM-2 and FSRS
  behaviour without always labelling which is which, so before implementing any
  scheduling/deck-options option, classify it SM-2 vs FSRS (see
  `server/DECK_OPTIONS_SUPPORT.md`, which marks SM-2-only options ⚪).
- **Centralize crate versions** in each workspace's `Cargo.toml`
  `[workspace.dependencies]` (the client already does; mirror it server-side).
- **Never commit secrets** — `.env` files are gitignored (see root
  `.gitignore`). Local config is supplied by the Nix dev shell
  (`DATABASE_URL`) and a code fallback (`JWT_SECRET`).
- **Database** — the server uses **Postgres** (provisioned by the flake via
  `nix run .#anjuman`). Migrations live in `server/migrations/` (consolidated
  `0001_create_schema.sql` + `0002_seed_data.sql`). Timestamps are
  `TIMESTAMPTZ` → `chrono::DateTime<Utc>`; JSON is `JSONB`; enum columns map
  to server-local `#[derive(sqlx::Type)]` enums in `src/db_types.rs` — read
  them via a `col as "col!: DbType"` type override, and write them by binding
  `DbType::X.as_str()` with a `$1::text::<enum>` SQL cast (sqlx's compile-time
  `query!`/`query_as!` macros can't map custom enums directly).
- **OpenAPI** — generated from `anjuman_contracts` via `utoipa`; never
  hand-edit a spec file.
- Generated code is gitignored (`shells/*/generated/`, `shells/leptos/dist/`).

## 6. Definitions of done (for agents)

When you finish a ROADMAP sub-task, you are done only when:

1. `cargo build` and `cargo test` pass in the affected workspace(s).
2. For server changes: the OpenAPI spec still generates (the `server` build
   does this automatically) and route coverage is unchanged.
3. No new warnings beyond those already documented.
4. Each acceptance criterion of the story has a `server/tests/` (or `client`
   `CruxCore`) test named after it, per `PLANNING.md`.
5. The corresponding ROADMAP checkbox is flipped to `[x]` (or the status
   marker updated) by *you*, with a one-line note of what changed.
