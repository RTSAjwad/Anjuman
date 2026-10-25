# Carve-out (shell agent → core agent): expose "decks loaded" for deep-link hydration

**From:** shell agent
**To:** core agent
**Status:** pending
**Traceability:** `docs/client/REACT_SHELL_PLAN.md` §5 Phase 4 (deferred decision), `client/shells/react/app/lib/use-hash-router.ts`.

## Problem

The React shell's hash-router hydrates a deep link (`#/decks/42/study`, `#/cards`,
…) once on load, by mapping the initial hash back to `Event::OpenDeck`/`Show*`.
To fire `OpenDeck` correctly, the deck list must already be loaded (the core's
`OpenDeck` is a no-op for an unknown id, and the deck tree id must exist before it
can be opened).

The shell currently gates hydration on `vm.decks.is_empty()` as a *proxy* for
"decks have finished loading":

```ts
decksLoaded: view.decks.length > 0
```

This is wrong for a user who genuinely has **zero decks** (a brand-new account):
`decks` is empty both *during* the fetch and *after* a successful zero-deck fetch,
so hydration never fires and the deep-link / URL mirror stays dormant for that
user.

## What I need

A reliable "the decks fetch has completed" signal on the `ViewModel`, **separate
from the deck list length**. The natural shape (matching `decks_error`):

- `pub decks_loaded: bool` on `ViewModel` — set `true` once the core has applied a
  `DecksResult` (success **or** error), and reset to `false` on `Logout` /
  re-authentication / whenever a fresh `DecksRequested` is in flight. (The
  `decks_error` field already tracks the failure; the shell just needs the
  "settled" bit distinct from "has rows".)

Minimal alternative (if a new field is undesirable): document an existing field
the shell can use instead to distinguish "still loading" from "loaded, empty".
But a dedicated `decks_loaded: bool` is the cleanest and least ambiguous.

## Why this is a carve-out (not self-servable)

`ViewModel` lives in `client/shared/src/app.rs` — core-agent scope per `AGENTS.md`
§2. The shell agent cannot add the field itself.

## Impact / validation

- A `#[cfg(test)]` test in `shared` asserting `decks_loaded` flips `false→true`
  after a `DecksResult` (both Ok and Err), and resets on `Logout`, per the DoD.
- Shell then reads `view.decks_loaded` (not `decks.length > 0`) in
  `use-hash-router.ts`, fixing deep-link hydration for 0-deck users.
- No `contracts`/`server` change expected; this is a core-local `ViewModel` field.
