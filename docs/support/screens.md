# Anjuman Client — Screens & Client-side Behaviour Matrix

The client analog of [`docs/support/deck-options.md`](./deck-options.md) / [`docs/support/preferences.md`](./preferences.md).
It catalogues what the **client** owns — screens, flows, and the behaviours that
stages 2–3 marked "client-side (stage 4)" — so each can become a `US-4.x` story
in `ROADMAP.md` stage 4. The *ordering* and *status* live in [`docs/process/ROADMAP.md`](../process/ROADMAP.md); this
document records the *what* and *why*.

Legend:

- ✅ implemented
- 🟡 partial
- ❌ not yet implemented (a real client-side gap)
- ⚪ not applicable / deferred to a later stage

## Scope: core vs. shell

- **Core (`client/shared/`)** owns all logic and is where acceptance criteria are
  tested (`CruxCore::update` + `caps.effects()`). It consumes `anjuman_contracts`
  DTOs over `crux_http` (JSON) and stores the JWT via `crux_kv`; `crux_time` is
  not yet a dependency (added when a story needs it).
- **Shell (`client/shells/leptos/`)** renders the `ViewModel`, emits `Event`, and
  executes `Effect`s. Kept thin; generally not unit-tested.

## Screens & flows (stable screens first)

| Screen / flow | Status | Notes |
|---|---|---|
| Auth (login → store JWT → `/me`) | ✅ | The prerequisite for everything else; US-4.2. |
| Decks (list/detail) | 🟡 | US-4.3 core done (list + nested tree + counts); shell rendering pending the shell agent. Detail is a later story. |
| Notes (browser/edit) | ❌ | Consumes `GET /notes`, note-type/template UI later. |
| Cards / browser | ❌ | Consumes `GET /cards`; flag/suspend/bury actions. |
| Study session | ❌ | Consumes `GET/POST /decks/{id}/study`; the single-card loop. |
| Deck options / preferences settings | ❌ | CRUD `deck-options` + `/preferences` in UI. |
| Classes / dashboard (teacher) | ❌ | Consumer of `/classes`, `/dashboard`; lower priority. |
| Tagging / search | ❌ | Deferred to stage 6 (search) — no client work now. |

## Shell coverage (per-platform parity)

Stories target the **core**; each carries a `Shell contract` checklist that every
shell satisfies. This table is the single parity ledger — one ✅/❌/⚪ cell per
screen/behaviour per shell. When a new shell lands, check off its column against
the existing shell contracts (do **not** author parallel stories). Form-factor-
specific items (answer keys, "spacebar also answers", theme) are shell-owned and
tracked here only.

| Screen / behaviour | Leptos (now) | SwiftUI | WinUI | Compose | Libadwaita |
|---|---|---|---|---|---|
| Auth (login / restore session / `/me`) | ✅ | ⚪ | ⚪ | ⚪ | ⚪ |
| Decks | ✅ | ⚪ | ⚪ | ⚪ | ⚪ |
| Notes | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Cards / browser | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Study session | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Deck options / preferences settings | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Classes / dashboard | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| On-screen timer | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Audio playback | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Auto-advance | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Timebox popup | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Leech "Tag Only" popup | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |
| Theme (dark/light/follow-system) | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |

> **Theme + card dark-mode decision (US-4.6):** dark mode on *cards* is Anki's
> **CSS inversion** (`night_mode` class + `filter: invert(…)`), not a palette
> re-theme — card authors write arbitrary CSS that a token swap can't cover. The
> shell injects the inversion into the card `<iframe>`; the core/server only
> deliver `StudyCardView.styling`. See US-4.6 in [`docs/process/ROADMAP.md`](../process/ROADMAP.md).
| Answer-key bindings | ⚪ | ⚪ | ⚪ | ⚪ | ⚪ |

Legend: ✅ implemented · 🟡 partial · ⚪ not yet / shell not built.

## Client-side behaviours deferred from stages 2–3

These were persisted server-side (or descoped) with a "behaviour client-side
(stage 4)" note. Each becomes its own story once the stable screens land:

| Behaviour | Source | Server state (already done) | Client work (stage 4) |
|---|---|---|---|
| On-screen timer | US-2.14 | `show_on_screen_timer`, `stop_timer_on_answer` booleans | Render the running timer; stop on answer. |
| Audio playback | US-2.14 | `dont_play_audio_automatically`, `skip_question_when_replaying_answer` | Play/stop audio; Replay action honours the toggles. |
| Auto-advance | US-2.14 | `auto_advance_*` (seconds + question/answer actions) | Count down and apply the configured action. |
| Maximum answer seconds | US-2.15 | `maximum_answer_seconds` (server clamps) | Run the stopwatch and send `response_time_ms`. |
| Timebox popup | US-3.2 | `timebox_time_limit` (persisted, persistence-only) | Periodically show "N cards this timebox". |
| Leech "Tag Only" popup | US-2.3 | `leech_action` enum (`TagOnly` is a no-op server-side) | Show the leech pop-up (tagging itself is stage 6). |
| Theme / answer keys / form-factor prefs | stage 3 matrix | n/a (client-owned) | Client-local settings (see [`docs/support/preferences.md`](./preferences.md)). |

## Client-side divergences to preserve (from stages 2–3)

- **Deck-tree collapse state is not persisted (client-only, transient).** Anki
  remembers each deck's expanded/collapsed state across sessions. Anjuman's
  Leptos shell currently **collapses all decks by default** on every load and
  does **not** persist the user's expand/collapse choices. This is a deliberate
  simplification until a real decision is made. It could later be resolved as:
  (a) *ignored* — always collapsed (simplest, but loses Anki parity);
  (b) *client-specific* — persist the open-item set in `localStorage` via the
  existing KV capability, or a dedicated per-shell setting; or
  (c) *server-specific* — persist per-user-per-deck open state server-side,
  synced across shells (heavier, but matches Anki's cross-device behaviour).
  Until then, this is tracked here, not as a core story.
- **Random display orders are deterministic per student-day** (not per-session),
  because the server is stateless — recorded in [`docs/support/deck-options.md`](./deck-options.md); the
  client must not assume per-session shuffling.
- **`Mix` new/review order was descoped** to "show after" (server), documented as
  a divergence; a faithful `Mix` would need session state the server doesn't hold.
- **FSRS simulators / Help Me Decide** are descoped to a **post-client stage** —
  no client work now.

## Deferred entirely (not client, not now)

- Collection-wide scoping questions (stage 7): `new_cards_ignore_review_limit`,
  `limits_start_from_top`, desired-retention per-deck, custom scheduling (JS).
- FSRS optimizer (US-2.18b) — deferred past stage 4 and stage 6.
