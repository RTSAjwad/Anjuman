# Planning, User Stories, and Test Strategy

How work is planned, documented, and ordered in this repo. Read this before
writing new user stories or adding tests.

---

## 1. Guiding division of responsibilities

Three documents, three jobs — keep them from overlapping:

| Artifact | File | Holds |
|---|---|---|
| **Order + story list** | [`ROADMAP.md`](./ROADMAP.md) | *What* is next, in dependency order, with status |
| **Feature spec** (the "why") | `server/DECK_OPTIONS_SUPPORT.md`, `server/PREFERENCES_SUPPORT.md` | The detailed per-feature support matrix that stories are *derived from* |
| **Test contract** | `server/tests/*.rs`, `client/shared` tests | Executable acceptance criteria — the tests *enforce* the story |

Rule of thumb:

- **`ROADMAP.md`** is the single source of truth for *ordering* and *status*.
  It stays terse; each stage links to the spec that drives it.
- **Support matrices** capture *what* a feature should do (and what's still
  missing vs. Anki). They are never the ordering document.
- **Tests** are the *executable* form of a story's acceptance criteria.

---

## 2. User-story format

Stories live under a ROADMAP stage as checkboxes. A story is *not* a code task
("add `sort_clause` fn") — it's an outcome expressed from a persona's point of
view. Use this template, id stamped `US-<stage>.<n>`:

```markdown
### US-3.2 — Custom review order

**As** a student,
**I want** to choose how review cards are ordered (ascending/descending/random),
**so that** I control the order I encounter them.

**Acceptance criteria**
- [ ] `next_due_card` honours the preset's review-order setting.
- [ ] "random" produces a stable seed per study session (not per-card shuffle).
- [ ] `GET /deck-options` reflects the new field (OpenAPI regenerated).

**Out of scope**
- Global review-order (deck-level only for now).
```

Conventions:

- **Ground stories in the Anki manual.** Anjuman aims for maximum Anki
  compatibility, so every story that touches scheduling, deck options,
  preferences, or study behaviour must cite the relevant page of
  <https://docs.ankiweb.net/> and mirror its terminology and defaults. When the
  manual is silent or we deliberately diverge, say so explicitly.
- **Classify SM-2 vs FSRS before implementing.** Anjuman supports FSRS only,
  not SM-2. The manual interleaves the two without always saying so. Before
  writing a story for an option, determine whether it is SM-2-specific (then
  mark it ⚪/out-of-scope and skip it) or FSRS/algorithm-neutral (then implement
  it). Record the classification in the story and support matrix.
- **The id `US-<stage>.<n>`** is referenced in commits ("fixes US-3.2") and in
  test names, so a story, its tests, and its change are traceable.
- **Acceptance criteria are the contract.** Each `[ ]` must be observable and
  testable — if you can't write a test for it, it isn't an acceptance criterion.
- **Out of scope** is mandatory: it records the deliberate decision *not* to do
  something, so the descope survives future readers. (Mirrors the existing "record
  the descope decision in the matrix" rule.)

---

## 3. Acceptance criteria → tests

The acceptance criteria are the source of truth for what "done" means, and the
tests are the compiled version of those criteria.

- **Server stories (stages 2–3)** — each criterion maps to a test in
  `server/tests/`, using the `TestApp` harness (`server/tests/common/mod.rs`) and
  `tower::ServiceExt::oneshot`. Name the test after the criterion it asserts.
- **Client stories (stage 4)** — each criterion maps to a `CruxCore::update`
  test in `client/shared`, asserting on `caps.effects()` (the side-effect-free
  Elm-style idiom; `AppTester` is deprecated — see `client/AGENTS.md`).

### Client (Crux) test idiom

The client core is pure and side-effect-free, so **no server, network, or UI is
run during a client test**. A criterion is asserted by driving `update` directly
and inspecting the returned `Command` (its `effects()`) and the resulting
`Model`/`ViewModel`. This mirrors the existing counter tests in
`client/shared/src/app.rs`.

An HTTP round-trip is **two events**, and therefore two assertions:

1. The initiating event (e.g. `Event::LoadDecks`) emits an `Effect::Http`
   capability carrying the correct method/URL/headers — assert on
   `caps.effects()`.
2. The completion event (e.g. `Event::DecksReceived(Response)`) is fed a **canned**
   `HttpResponse` (success or error) as input; its `update` mutates `model` and
   yields a `Render` effect — assert on the resulting `Model`/`ViewModel`.

The wire payloads are `anjuman_contracts` DTOs, deserialized in the core (the
shell forwards bytes opaquely — see `client/ARCHITECTURE.md` §5).

A story is **done only when** every acceptance criterion has a passing test. The
converse applies too: **no untested acceptance criteria** — if a criterion isn't
worth testing, tighten or drop it.

### Core vs. shell test split

The client has two layers with very different testability:

- **The core (`client/shared/`)** owns all logic, state transitions, HTTP
  parsing, and capability sequencing. **Acceptance criteria belong here** and
  are tested as pure `update` tests (see the idiom above).
- **The shell (`client/shells/leptos/`)** only renders `ViewModel`, emits
  `Event`, and executes `Effect`s. It is kept deliberately thin and is generally
  **not unit-tested** (declarative UI; correctness is enforced by types). Reserve
  shell-side tests for genuinely tricky glue (e.g. the `core_link` effect loop) —
  do not gold-plate this layer.

Acceptance criteria should therefore be phrased as **core behaviours** ("on
`LoginSuccess`, `model.auth_token` is stored and an `Http` effect for `/me` is
emitted"), not UI prose ("the user sees a login form").

Rules carried over from `AGENTS.md` "Definitions of done":

1. `cargo build` and `cargo test` pass in the affected workspace(s).
2. Each acceptance criterion has a test named after it.
3. The ROADMAP checkbox is flipped (`[ ]` → `[~]` in progress, `[x]` done) with
   a one-line note.

---

## 4. Ordering

The repo already follows a clear rule — make it explicit and keep it:

> **Tasks 2/3 change the API → task 4 consumes that API → so 4 is last, and 2/3
> are not migrated twice.**

Within a stage, **front-load the API surface** (DTO + endpoint shape) before the
behavioural depth, because the client only needs the *shape* to start building
against it. Keep the client pinned to `anjuman_contracts` so the wire types act
as the shared interface between a finished backend story and an in-progress
frontend story.

---

## 5. Tests must not depend on seed data

`server/tests/` intentionally **does not** rely on `0002_seed_data.sql`. The
`TestApp` harness truncates the tables a test is expected to mutate and each test
seeds its own rows. Rationale: seed fixtures are order-dependent and drift as the
schema evolves; a test that creates its own fixture is self-contained and robust.

Exception: the harness preserves the id-0 global-default deck-options preset
because request handling depends on it unconditionally.
