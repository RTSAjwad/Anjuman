# PIPELINE — the SDLC contract

How a story moves from idea to "done" in this repo. The *what* and *why* of each
story live in [`ROADMAP.md`](./ROADMAP.md) and
[`PLANNING.md`](./PLANNING.md); this document pins down **who does what, in
what order**, and the gate a story must pass before it is declared complete.

> This contract applies whether you run two parallel Zed agents (core + shell)
> or any future orchestration. The **roles and write boundaries** are the
> invariant; the transport is not.

---

## 1. Roles and write boundaries

Two specialist agents plus a human coordinator. The boundary is a **write**
boundary and is the one thing that must never be crossed.

| Role | Owns (may edit) | Must never edit |
|---|---|---|
| **core** | `contracts/`, `server/`, `client/shared/`, workspace manifests, `docs/process/ROADMAP.md`, `docs/process/PLANNING.md`, `server/migrations/`, permissions | `client/shells/*` |
| **shell** | `client/shells/*` only (Leptos now; SwiftUI/WinUI/Compose/Libadwaita later) | `contracts/`, `server/`, `client/shared/` |
| **coordinator** (you) | the roadmap, the story list, the routing decisions; the DoD gate | — (does not edit code) |

Rules:

- **One agent holds the pen at a time.** A story is carried by exactly one agent
  from start to finish; there is no concurrent editing of the same scope.
- **The interface crosses the boundary, not the editor.** When a story changes
  the boundary (a new `Event`/`ViewModel`/`Effect` field, a contracts DTO), the
  core agent *finishes and records* the interface; the shell agent then consumes
  it. The shell never authors an interface it needs — it files a *carve-out*
  request (see §3).

## 2. The per-story loop

Every story, regardless of layer, follows the same loop:

1. **Split.** The coordinator splits a roadmap stage into stories
   (`US-<stage>.<n>`), each with an Anki citation and an SM-2/FSRS classification
   (see [`PLANNING.md`](./PLANNING.md) §2). Acceptable criteria are written at
   this point — a story without testable criteria is not ready.
2. **Route.** The coordinator routes each story by layer: core-only → core,
   shell-only → shell, boundary-spanning → **core first, then shell**.
3. **Plan.** The agent reads the relevant support matrix, plans the change, and
   confirms the classification and any divergence/descope *before* writing code.
4. **Test.** Each acceptance criterion becomes a named test (server:
   `server/tests/`; client core: `client/shared` `CruxCore::update`). See
   [`PLANNING.md`](./PLANNING.md) §3.
5. **Implement.** Minimal, focused change to satisfy the criteria.
6. **Gate.** The story passes the Definition of Done (§4) *before* its
   ROADMAP checkbox is flipped.

## 3. Carve-outs (boundary-spanning work)

A *carve-out* is a request from the shell agent to the core agent for an
interface change the shell needs but cannot make itself (it may not edit
`client/shared/` or `contracts/`).

The carve-out must state:

- **What** interface element is missing (the exact `Event`/`ViewModel`/`Effect`
  field or contracts DTO), and its desired shape.
- **Why** the shell needs it (the concrete rendering/execution gap).
- **Scope** — a hard promise that only the interface is requested, not shell code.

The core agent implements the interface (and its test), records the resulting
shape, and reports back. The shell agent then consumes it. A change that spans
the boundary is therefore always sequenced **core-first-then-shell**; the two
agents never work the *same* story in parallel.

## 4. Definition of Done (the gate)

A story is **done** only when all of the following hold:

1. `cargo build` and `cargo test` pass in the affected workspace(s).
2. For server changes: the OpenAPI spec still generates (the `server` build does
   this automatically) and route coverage is unchanged.
3. No new warnings beyond those already documented.
4. Each acceptance criterion has a `server/tests/` (or `client/shared`
   `CruxCore`) test **named after it** — one test per criterion, per
   [`PLANNING.md`](./PLANNING.md) §3.
5. The corresponding `ROADMAP.md` checkbox is flipped
   (`[ ]` → `[~]` while in progress, `[x]` when done) **by the coordinator**, with
   a one-line note of what changed.

No checklist item is negotiable; a story that cannot satisfy all five returns to
§2 rather than being marked done.

## 5. The staleness rule

`ROADMAP.md` and the support matrices mutate as work lands. Therefore:

- Re-read `ROADMAP.md` at **every** task boundary — do not trust a cached or
  startup snapshot of the roadmap or the matrices.
- An agent's initial context is a *snapshot*; the living source of truth is the
  file on disk at the moment of work.

## 6. Ordering invariants

Taken from [`PLANNING.md`](./PLANNING.md) §4 and kept here for the pipeline:

- **The API is front-loaded before behavioural depth** — the client only needs
  the *shape* to build against, so the DTO + endpoint shape lands first.
- **Stages 2/3 (server) change the API; stage 4 (client) consumes it** — so 4 is
  last, and 2/3 are not migrated twice.
- **The client is pinned to `anjuman_contracts`**, so wire types are the shared
  interface between a finished backend story and an in-progress frontend story.
