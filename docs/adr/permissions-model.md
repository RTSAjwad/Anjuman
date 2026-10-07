# ADR — Authorization & permissions model

**Status:** proposed (supersedes the *implicit* model; replaces rolling role checks)
**Date:** 2026-10-07
**Scope:** `contracts/`, `server/`, `client/shared/` (no shell work)

## Context

Anjuman authorizes requests in three layers that have grown organically rather
than by design:

1. **Authentication** — `server/src/auth/` (`Claims { sub, school_id, role }`) is
   solid and centralized.
2. **Role gates** — `if claims.role ∈ { Admin, Teacher }` re-expressed **~40 times**
   across ~9 handler files, each with a hand-rolled copy of
   `check_teacher_or_admin` / `check_admin`.
3. **Deck access** — `server/src/permissions.rs` (US-4.7) is a *resource-grant*
   model (`DeckPerm::{ Study, VisibleOnly, None }`), deliberately **not**
   role-gated, but it overlaps `decks.rs::check_deck_owner` /
   `check_deck_collaborator` (owner-or-admin) and is deck-only.

Two kinds of authorization are conflated:
- **Role gates** ("teachers/admins may manage decks/notes/tags/presets") — coarse,
  `if role` scattered.
- **Resource grants** ("may *this user* act on *this deck/note*") — centralized
  for study, but split between `permissions.rs` and `decks.rs` for governance.

A second role axis also exists (`class_members.membership_role`:
teacher/student *within* a class), tracked separately from the global
`users.role`, and currently overlapping in intent.

## Decision

Consolidate **all** authorization into `server/src/permissions.rs` behind two
layers, and reject the heavier alternatives (see "Consequences"):

1. **RBAC as a thin, fast, coarse layer** — keep `role` in the JWT for the cheap
   auth fast-path and coarse gates (`Admin` → manage any deck in school).
2. **Capability-named predicates (PBAC seam)** — replace the ~40 `if role`
   checks with functions that read as intent: `can_manage_decks(claims)`,
   `can_manage_notes(db, claims, deck)`, `can_manage_presets(claims)`,
   `can_administer_school(claims)`, …
3. **Generalized relationship model (ReBAC)** — extend the existing resource-grant
   (`deck_access`) so "governance vs study" is one enum (add `DeckPerm::Manage`),
   and route `check_deck_owner`/`check_deck_collaborator` through it, so "may this
   user act on this resource" is answered in one place by relationship
   (owner / collaborator / class member / ancestor / descendant), not by role
   label alone.

The transition is **Option A now, structured so B drops in later**: the
`has_permission`/capability seam is the *single* funnel every decision passes
through, so migrating role→permission to a database table (RBAC-with-permissions)
later is a swap of one function's implementation, not a handler sweep.

## Alternatives considered

- **Pure RBAC (status quo, formalized)** — rejected as the end-state: fine for
  coarse intent but cannot express per-resource sharing ("student of shared deck
  X", "teacher of class A but student in class B") without role explosion.
- **Permission table now (PBAC-backed)** — deferred, not adopted yet: correct
  end-state but adds a migration + seed + full call-site rewrite now, with no
  current requirement for mutable role→permission mapping. The seam keeps this
  ready.
- **ABAC (attribute policies)** — **rejected as a non-goal.** No cross-cutting
  attribute-policy requirement exists today (no "subject.clearance ≥
  resource.classification AND time in hours" rules); every decision reduces to a
  role or a relationship. Introducing a policy engine (OPA/XACML/casbin) would
  add a dependency + a second source of truth for rules with zero payoff, and
  would fight the named-predicate, one-test-per-criterion SDLC. Only revisit if a
  *per-school-configurable policy* requirement ever materializes.

## Consequences

- **Positive:** one place to change a rule; call sites read as intent; deck
  governance and study share one enum; no schema change required; the seam makes
  a future permission table a contained edit.
- **Negative:** a large but mechanical refactor touching every handler; risk of
  subtle behavior drift mitigated by the "no behavior change, route coverage
  unchanged" DoD and a named test per capability.
- **Non-goal (explicit):** no ABAC/policy engine; no DB permission table in this
  pass; no removal of the JWT `role` claim.

## Open questions (carried into the stage's design section)

1. Capability-naming convention (resource-first `can_manage_notes` vs.
   action-first `can_notes_manage`).
2. Whether `DeckPerm` gains a `Manage` level now, or deck *governance* stays
   separate from deck *study* for this pass.
3. Whether `class_members.membership_role` should be folded into the same
   capability model or kept as a distinct axis.
