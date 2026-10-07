# ADR — Authorization & permissions model

**Status:** accepted (implemented — stage 9)
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
   checks with functions that read as intent: `can_manage_decks`, `can_manage_notes`,
   `can_manage_presets`, `can_administer_school`, … (resource-first naming).
3. **Generalized relationship model (ReBAC)** — extend the existing resource-grant
   (`deck_access`) so "governance vs study" is expressed as **two orthogonal
   dimensions**, not one ordering:
   - **Study** (`DeckPerm::{None, VisibleOnly, Study}`) — grant-based,
     role-agnostic (`deck_permission`); unchanged from US-4.7.
   - **Governance** (`DeckGovernance::{None, Contribute, Manage}`) — role-gated
     AND relationship-gated (`deck_governance`): `Manage` = owner (teacher/admin)
     or admin-trump; `Contribute` = collaborator (teacher/admin).

   These are **not** a single ordering: an admin may `Manage` a deck they cannot
   `Study` (the admin trump grants governance, not study — confirmed by the
   existing US-4.7 tests). A student owner/collaborator gets *study* only,
   never governance, because governance is role-gated.

Every decision is **capability (role bundle) AND relationship (user ↔ resource)**;
a role never grants a *particular* resource, only the *kind of action* that role
may take on resources it is related to in a specific way.

**Admin trump (deck capabilities): kept.** `Admin → Manage` on any deck in school,
skipping the owner/collaborator relationship check — matching current
`check_deck_owner` behavior. Admins are **not** trumped for *study* (the existing
`deck_access` has no admin shortcut and keeps none).

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

## Open questions

1. ~~Capability-naming convention~~ → **resource-first** (`can_manage_notes`).
- ~~Governance levels~~ → **two orthogonal dimensions**: `DeckPerm` (study:
   `None/VisibleOnly/Study`) + `DeckGovernance` (management: `None/Contribute/Manage`).
   Admin trump grants `Manage` but **not** `Study` — governance and study are
   orthogonal, matching the existing US-4.7 tests.
3. ~~Admin trump~~ → **kept** (admin manages any deck in school).
4. Membership second axis (`class_members.membership_role` vs. global `users.role`)
   — kept distinct for now; revisit when class-scoped management appears.
5. Role in JWT vs DB — kept in JWT for this pass; the capability seam makes a
   later DB-backed move a one-function swap.
