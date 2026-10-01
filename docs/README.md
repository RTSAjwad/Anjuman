# Anjuman — documentation index

All project documentation lives under this `docs/` directory. Start here; then
follow the pointers below.

> The repo-root [`AGENTS.md`](../AGENTS.md) carries the **agent roles, write
> boundaries, and conventions** (auto-discovered by Zed/tooling). The detailed
> documentation lives here under `docs/`, indexed below.

---

## The process documents (read these first)

| Document | What it's for |
|---|---|
| [`process/ROADMAP.md`](./process/ROADMAP.md) | The **ordered task list** — single source of truth for *what's next* and *status*. |
| [`process/PLANNING.md`](./process/PLANNING.md) | The **user-story format**, acceptance-criteria→test mapping, and ordering rules. |
| [`process/PIPELINE.md`](./process/PIPELINE.md) | The **SDLC contract** — who does what, in what order, and the Definition-of-Done gate. |

## The support matrices (feature specs)

These capture *what* a feature should do (and what's still missing vs. Anki).
Stories in `ROADMAP.md` are derived from them.

| Matrix | Drives | Covers |
|---|---|---|
| [`support/deck-options.md`](./support/deck-options.md) | roadmap task 2 | Anki deck-options feature parity (FSRS vs SM-2 classification). |
| [`support/preferences.md`](./support/preferences.md) | roadmap task 3 | Anki preferences feature parity (server-side subset). |
| [`support/screens.md`](./support/screens.md) | roadmap task 4 | Client screens + client-side behaviour matrix. |

## Client architecture & guides

| Document | What it's for |
|---|---|
| [`client/ARCHITECTURE.md`](./client/ARCHITECTURE.md) | Client design rationale (core-first, thin shells, FFI boundaries). |
| [`client/CLIENT.md`](./client/CLIENT.md) | Client-specific version pinning and gotchas. |
| [`client/SHELLS.md`](./client/SHELLS.md) | The shell implementation guide (capability + effect-resolve patterns). |
| [`../client/README.md`](../client/README.md) | Client build/run/prerequisites quick reference. |

## Architecture decision records (ADRs)

| Document | Decision |
|---|---|
| [`adr/postgres-migration.md`](./adr/postgres-migration.md) | SQLite → Postgres migration plan. |
| [`adr/timestamptz-migration.md`](./adr/timestamptz-migration.md) | TIMESTAMPTZ migration sub-plan. |

---

## Authoritative external reference

**Anki manual** — the authoritative reference for feature behaviour, since
Anjuman aims for maximum Anki compatibility:

- Root: <https://docs.ankiweb.net/>
- Deck options: <https://docs.ankiweb.net/deck-options.html>
- Preferences: <https://docs.ankiweb.net/preferences.html>
