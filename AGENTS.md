# AGENTS.md

Authoritative guidance and onboarding for any agent (or human) working on this
repository. This is the **single agent file** — the repo-root `AGENTS.md` that
Zed and git-aware tooling auto-discover. All detailed documentation lives under
[`docs/`](./docs); start at [`docs/README.md`](./docs/README.md) for the full map.

---

## 1. What this repo is

**Anjuman** — an Anki-inspired spaced-repetition platform with organisation and
collaboration (classes, roles, shared decks). One git repository containing
three independent Cargo workspaces, linked by path dependencies:

| Folder | Crate | What it is |
|---|---|---|
| `contracts/` | `anjuman_contracts` | The **single source of truth** for wire DTOs (request/response + shared enums like `UserRole`). Consumed by both the server and the client. |
| `server/` | `anjuman_server` | The Axum backend (HTTP API, FSRS scheduling, Postgres). |
| `client/` | (workspace) `shared` + `shells/*` | The Crux cross-platform app (Leptos shell implemented; SwiftUI/WinUI/Compose planned). |

> Note: these are **separate Cargo workspaces**, not one top-level workspace.
> This is deliberate — the client needs a WASM/size-optimized release profile,
> the server a standard throughput profile, and separate `/target` and
> `Cargo.lock` per workspace. See [`docs/client/ARCHITECTURE.md`](./docs/client/ARCHITECTURE.md) §2
> and §4 for the boundary model. The server and client consume
> `anjuman_contracts` via `path = "../contracts"`.

---

## 2. Agent roles and write boundaries

Two specialist agents plus a human coordinator. The boundary is a **write**
boundary and is the one thing that must never be crossed.

| Role | Owns (may edit) | Must never edit |
|---|---|---|
| **core** | `contracts/`, `server/`, `client/shared/`, workspace manifests, `docs/process/ROADMAP.md`, `docs/process/PLANNING.md`, `server/migrations/`, permissions, support matrices | `client/shells/*` |
| **shell** | `client/shells/*` only (Leptos now; SwiftUI/WinUI/Compose/Libadwaita later) | `contracts/`, `server/`, `client/shared/` |
| **coordinator** (you) | the roadmap, the story list, the routing decisions; the DoD gate | — (does not edit code) |

Rules:

- **One agent holds the pen at a time** — a story is carried by exactly one agent
  from start to finish.
- **The interface crosses the boundary, not the editor.** A change to the
  boundary (new `Event`/`ViewModel`/`Effect` field, a contracts DTO) is made by
  the core agent and *recorded*; the shell agent then consumes it. The shell
  never authors an interface it needs — it files a carve-out (see
  [`docs/process/PIPELINE.md`](./docs/process/PIPELINE.md) §3).

The full SDLC contract (per-story loop, carve-outs, definition of done, ordering
invariants) is in [`docs/process/PIPELINE.md`](./docs/process/PIPELINE.md).

---

## 3. Role → what to read (routes when your role is assigned)

You will be told which role you are. Read the docs for **your** role; the rest
is optional context.

### All agents (read first)

- [`docs/README.md`](./docs/README.md) — the doc index.
- [`docs/process/PLANNING.md`](./docs/process/PLANNING.md) — how stories are written and tested (AC → tests).
- [`docs/process/PIPELINE.md`](./docs/process/PIPELINE.md) — the SDLC contract and the Definition of Done.
- [`docs/process/ROADMAP.md`](./docs/process/ROADMAP.md) — what's next / current work (re-read at each task boundary; it mutates).

### Core agent (contracts / server / client-shared)

- [`docs/support/deck-options.md`](./docs/support/deck-options.md) — deck-options feature matrix (roadmap task 2).
- [`docs/support/preferences.md`](./docs/support/preferences.md) — preferences feature matrix (task 3).
- [`docs/client/ARCHITECTURE.md`](./docs/client/ARCHITECTURE.md) — the client core's design (you own `client/shared/`).
- [`docs/client/CLIENT.md`](./docs/client/CLIENT.md) — client version pinning + gotchas (you touch `shared/`; the shell touches `shells/`).

### Shell agent (client/shells/*)

- [`docs/client/SHELLS.md`](./docs/client/SHELLS.md) — the shell implementation guide (authoritative).
- [`docs/client/ARCHITECTURE.md`](./docs/client/ARCHITECTURE.md) — the *why* behind the core you consume.
- [`docs/client/CLIENT.md`](./docs/client/CLIENT.md) — version pinning, getchas, and the core-vs-shell split.
- [`docs/support/screens.md`](./docs/support/screens.md) — the screen/behaviour matrix (your "Shell coverage" column).

### Coordinator (you)

You drive the process; you do not edit code. Read §2 above and
[`docs/process/PIPELINE.md`](./docs/process/PIPELINE.md), and refer to the support
matrices when splitting stages into stories.

---

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
Postgres. They use a dedicated test database (`ANJUMAN_TEST_DATABASE_URL`,
default `postgres://127.0.0.1:5432/anjuman_test`) that the harness migrates and
resets per test — so Postgres must be running (`nix run .#anjuman`) and the
test DB must exist (`createdb anjuman_test`). See `server/tests/common/mod.rs`.

---

## 5. Working conventions

- **FSRS-only, no SM-2.** Anjuman supports the FSRS scheduler only; do not
  implement SM-2-specific features. Before implementing any scheduling/deck-options
  option, classify it SM-2 vs FSRS (see [`docs/support/deck-options.md`](./docs/support/deck-options.md),
  which marks SM-2-only options ⚪).
- **Centralize crate versions** in each workspace's `Cargo.toml`
  `[workspace.dependencies]`.
- **Never commit secrets** — `.env` files are gitignored. Local config comes
  from the Nix dev shell (`DATABASE_URL`) and a code fallback (`JWT_SECRET`).
- **Database** — Postgres (provisioned via `nix run .#anjuman`); migrations in
  `server/migrations/`; timestamps `TIMESTAMPTZ`, JSON `JSONB`, enums map to
  server-local `#[derive(sqlx::Type)]` enums (see the enum read/write cast
  pattern in the server source).
- **OpenAPI** — generated from `anjuman_contracts` via `utoipa`; never hand-edit.
- **Commit messages** are Conventional Commits scoped by layer — `type(scope): summary`,
  scope ∈ `core`/`server`/`contracts`/`db`/`shell/<name>`. Include the story id
  when relevant. Reserve Git tags for release markers.

---

## 6. Definition of Done (for agents)

Full detail in [`docs/process/PIPELINE.md`](./docs/process/PIPELINE.md) §4. A story is done only when:

1. `cargo build` and `cargo test` pass in the affected workspace(s).
2. For server changes: the OpenAPI spec still generates and route coverage is unchanged.
3. No new warnings beyond those already documented.
4. Each acceptance criterion has a `server/tests/` (or `client/shared` `CruxCore`)
   test **named after it**.
5. The corresponding `ROADMAP.md` checkbox is flipped by the coordinator, with a
   one-line note.
