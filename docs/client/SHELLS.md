# Shell Implementation Guide

A shell is the thin layer that turns the shared core's `ViewModel`, `Event`, and
`Effect` into a visible, interactive app. This document is the **authoritative
reference for building a shell** — any of the five targets (Leptos, SwiftUI,
WinUI, Jetpack Compose, Libadwaita). It records the shared *pattern*; per-shell
toolchain gotchas live in each shell's own directory when they genuinely diverge
(see [`CLIENT.md`](./CLIENT.md) for the shared version pinnings).

Read [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the *why* (core-first,
boundaries) and [`CLIENT.md`](./CLIENT.md) for the version-pinned *how*.
This guide is the *what a shell must do*.

## 1. The two access paths — pick one per shell

| Shell kind | Examples | How it drives the core |
|---|---|---|
| **Rust** | Leptos, Libadwaita | The typed `Core<Anjuman>` API in-process: `core.process_event(event)` → effects → `core.view()`. **No serialization.** |
| **Non-Rust** | SwiftUI, WinUI, Compose | The serialized FFI `Bridge` (`shared/src/ffi.rs`, `CoreFfi`) via `BoltFFI`-generated bindings + `Facet` typegen. |

Rust shells must **not** serialize through the FFI `Bridge`; they hold `Rc<Core<Anjuman>>` and call the typed API directly (see the Leptos `core_link.rs`).

## 2. The shell's three responsibilities

A shell is *only* glue. It does three things, nothing more:

1. **Render** the `ViewModel` returned by `core.view()`.
2. **Forward** user actions as `Event`s via `core.process_event(event)`.
3. **Execute** the `Effect`s the core requests, feeding results back via
   `core.resolve(request, payload)`.

There is **no logic in the shell** — no scheduling, no auth decisions, no JSON
parsing of HTTP bodies (the core deserializes). If you find yourself writing
business logic in a shell, it belongs in `client/shared`.

## 3. Routing & navigation (core-owned)

A shell **never holds its own routing state**. The core owns the route and it is
represented in the `ViewModel`, so every shell renders `view()` identically and
forwards the same `Event`s. The concrete rule:

- **The current screen/route is `ViewModel.route`.** It is a core-local `Facet`
  enum `CurrentRoute::{ DeckList, Cards, Notes, Study, DeckOptions }` — the single
  flat source of truth for the top-level destination. A shell renders exactly one
  branch from `match vm.route { … }` and must **not** introduce its own
  `active_view`/`nav_selected`/`route` signal (a `String`-based shell-local route
  drifted here for US-4.12/US-4.13 and was removed). The deck selection fields
  (`ViewModel.selected_deck` + `selected_deck_view`) remain for the two deck
  panels, but the authoritative "which top-level screen" is `route`.

- **Navigation is an `Event`, and the core chains the follow-up fetch.** Opening
  a deck's Study view is `Event::OpenDeck { deck_id }` (sets `route = Study`);
  opening its Options view is `Event::OpenDeckOptions { deck_id }` (sets
  `route = DeckOptions`). Top-level navigation is `Event::ShowDeckList` /
  `ShowCards` / `ShowNotes` — each **wins over an open deck** by clearing the
  deck selection + study state (like `CloseDeck`). The core auto-chains
  `StartStudy` (for `OpenDeck`) or `DeckOptionsRequested` (for
  `OpenDeckOptions`) and returns a `Render`. A shell therefore **forwards the
  event only** — it does not fire `StartStudy`/`DeckOptionsRequested` itself, or
  the fetch would happen twice.

- **Back-navigation resets the route in the core.** Leaving a deck is
  `Event::CloseDeck` (sets `route = DeckList`); the core clears `selected_deck`
  and resets `selected_deck_view` to `Study`. The shell does not need to reset
  anything — the next `OpenDeck`/`OpenDeckOptions` lands on the requested panel.

- **Transient UI state stays shell-local.** While the *route* belongs to the
  core, purely presentational state that no behaviour depends on (e.g. the
  Leptos shell's card `revealed` flag, an expanded/collapsed tree) may live in
  the shell. The test: if another shell can't see it and no `Event`/`ViewModel`
  field needs it, it may be shell-local; anything that affects *what the user
  can do* (which screen, which deck) is core-owned (i.e. `route`).

This is the Crux-idiomatic answer to routing: URLs/`leptos_router` are web-only
and would duplicate core state. The five shells share one routing model carried
by the `ViewModel` + `Event`s, not per-shell URL frameworks.

## 4. Feature gating (permissions)

The core resolves each user's capabilities server-side (`US-9.5`) and exposes
them as `ViewModel.permissions` (a `Facet`-friendly struct of booleans). The
shell's job is **UX gating only** — hiding (or dimming/locking) what the user
isn't allowed to do. The server enforces every permission regardless; the client
must merely mirror it so the two never drift.

- **Read `permissions`, never re-derive a role.** The shell must not hard-code
  `is_teacher`/`is_admin` checks or inspect `UserRole`. The only source of truth
  is `ViewModel.permissions.*`: `can_administer_school`, `can_manage_users`,
  `can_manage_decks`, `can_manage_notes`, `can_manage_presets`,
  `can_manage_note_types`, `can_manage_tags`, `can_manage_classes`,
  `can_view_dashboard`, `can_view_analytics`.

- **Study is a separate layer.** Study is per-deck, gated by
  `DeckSummary.studyable` — **not** in `permissions`. Never conflate the two:
  anyone with a study grant has `studyable == true` regardless of role.
  Study-side card actions (bury/suspend/reschedule/flag) are student-per-card and
  are therefore governed by study access, not the management permission bundle.

- **Hide vs. lock.** For management affordances, prefer *hiding* what the user
  can't do (e.g. an "assign preset" picker). Where a read-only *view* should
  remain reachable (e.g. the deck-options preset), keep the view but lock the
  edit/save controls — dim the form and omit the Save button, with a
  "you don't have permission" hint. This mirrors the existing `studyable`
  disabled-row pattern.

- **If a capability is missing, file a carve-out.** Do not invent a shell-local
  permission check. If the `ViewModel` lacks a field a new affordance needs,
  request it from the core agent rather than approximating it in the shell.

- **New screens inherit the pattern.** When a management screen (decks, notes,
  note-types, tags, classes, users, dashboard/analytics) lands, thread
  `vm.permissions` the same way: gate each affordance on the matching `can_*`
  field. Since `PermissionsView` is server-derived, a future DB-backed
  permission table will not require shell changes.

## 5. The effect loop (canonical shape)

Every shell implements this loop. The Leptos example below is the reference
implementation; translate the `task::spawn_local` + `async` to each platform's
concurrency idiom (Swift `Task`, Kotlin `viewModelScope` + `suspend`, C# `async`).

```rust
fn process_effect(core: &AppCore, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }
        Effect::Http(mut request) => {
            task::spawn_local({
                let core = core.clone();
                async move {
                    let response = http::request(&request.operation).await;
                    for effect in core.resolve(&mut request, response.into()).expect("resolve http") {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }
        Effect::KeyValue(mut request) => {
            task::spawn_local({
                let core = core.clone();
                async move {
                    let result = kv::execute(&request.operation).await;
                    for effect in core.resolve(&mut request, result).expect("resolve kv") {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }
    }
}
```

Key points:

- `process_effect` takes `Effect` **by value** (not `&Effect`) because `resolve`
  needs to move the request.
- The effect payload is a `crux_core::Request<Op>`; the *actual* operation is at
  `request.operation`.
- `resolve` returns a new batch of effects (usually just `Render`); recurse over
  them, don't drop them.

## 6. Implementing each capability

### HTTP (`crux_http`)

The shell performs the fetch and returns a `crux_http::Result<HttpResponse>`
(`HttpResponse::status(status).body(bytes).build()`). Reference:
`client/shells/leptos/src/http.rs` (uses `gloo-net` — `Request::get/post(url)`,
`.header(name, value)`, `.send()`, `response.binary()`, `response.status()`).

- The core sends a `crux_http::protocol::HttpRequest` (plain `method`/`url`/
  `headers`/`body` as `String`/`Vec<u8>`). Validate nothing — pass through to the
  native client.
- Handle at least `GET` and `POST`; add more verbs as the core needs them.
- Errors map to `HttpError::Io(String)` (or the platform-appropriate variant).

### Key-value (`crux_kv`)

The shell persists bytes and returns a `crux_kv::KeyValueResult` (`Ok { response:
KeyValueResponse::Get/Set/Delete { .. } }`). Reference:
`client/shells/leptos/src/kv.rs` (uses `gloo_storage::LocalStorage`).

- `Get { key }` → `KeyValueResponse::Get { value: Value::Bytes(v) | Value::None }`.
- `Set { key, value }` → store, return `KeyValueResponse::Set { previous }`.
- `Delete { key }` → remove, return `KeyValueResponse::Delete { previous }`.
- On platform error, return `KeyValueResult::Err { error: KeyValueError::Other { .. } }`.

### Time (`crux_time`) — when added

`crux_time` is not yet a dependency. When a story needs it, the shell supplies
platform time and the pattern is identical: an `Effect::Time(op)` variant resolved
with the platform's clock.

## 7. Where a shell's work is *specified*

A shell agent does **not** read the user stories and invent behaviour — it
satisfies the **shell contract** and the **coverage matrix**:

- **`ROADMAP.md`** ([`docs/process/ROADMAP.md`](../process/ROADMAP.md)) — each story ends with a `Shell contract` checklist (render X,
  forward Y, execute Z). That is the shell's work order.
- **`SCREENS_SUPPORT.md`** ([`docs/support/screens.md`](../support/screens.md)) — the "Shell coverage" matrix is the parity
  ledger; tick your shell's column as you complete a contract.

If the contract is silent on something a shell needs (an ambiguous prop, a
missing event), ask — don't invent a convention. The contract is the interface
between the core agent and the shell agent.

## 8. Per-shell gotchas (live here, not in the shared docs)

- **Leptos** — CSR-only single binary (no `lib` target). Uses Tailwind + daisyUI
  (see §9 for the styling contract) with `gloo-net` + `gloo-storage`.
- **SwiftUI / WinUI / Compose** — FFI shells; see [`CLIENT.md`](./CLIENT.md) §BoltFFI for
  the binding + typegen flow (`boltffi generate <lang>`).

Add a section here for each shell as it is implemented.

## 9. CSS scoping & self-containment (Leptos + Tailwind)

The Leptos shell styles with **Tailwind utilities + shadcn convention theme
tokens** — the same global-utility model rust-ui/leptos-ui (and shadcn, which it
ports) uses by default: components are unstyled-scope-free class strings resolved
against a global design system. Consistency comes from **`@layer` discipline**,
not from scoped styles:

- **Page/layout CSS** lives in `input.css` under `@layer components` (moved out
  of the former unlayered `index.html` `<style>` block). This places it between
  Tailwind's `base` and `utilities` layers, so it can no longer silently out-rank
  preflight and leak into component internals.
- **Components own their styling via explicit utility classes.** Every element a
  component renders carries its own reset (`list-none m-0` on a popover list,
  `p-*` on items, …) so ancestor reach-in cannot change its spacing — the
  component is self-contained *by construction*, with no `<style>` injection and
  no scoping (rust-ui/shadcn assume this).

This is the shadcn model exactly: global utilities for skin, `@layer` for
structure, and no CSS-in-Rust scoping of any kind.

### 9.1 Two distinct CSS mechanisms (don't conflate them)

1. **Inheritance.** Certain properties inherit *through the DOM tree*: `color`,
   `font-*`, `text-align`, `visibility`, and **all CSS custom properties**
   (`--foo`). This is real inheritance and it is a *feature*: it is why a
   component picks up the active daisyUI theme tokens (`--color-base-content`,
   etc.) with no effort. Layout/box properties (`display`, `position`, `margin`,
   `padding`, `width`, `overflow`, `z-index`, flex settings) **do not inherit**.

2. **Selector reach-in.** Leptos components compile to plain DOM elements with
   **no DOM isolation**. A CSS selector matches ancestors/descendants across
   component boundaries, so a global reset (`button { … }`) or a descendant rule
   (`nav li { … }`) reaches *into* a component only when that component is
   mounted inside the matching ancestor. This — not inheritance — is what made
   identical markup render differently in the study navbar vs. the card
   inspector.

### 9.2 Why daisyUI semantic classes cause drift

DaisyUI's "semantic" components (`btn`, `menu`, `dropdown`, `navbar`, …) are
pre-built class bundles that fight idiomatic Leptos/Tailwind in concrete ways:

- **Mega-selectors with descendant combinators** (`menu :where(li > button)`,
  `dropdown :focus-within .dropdown-content`). They reach into components and are
  coupled to internal DOM structure; a one-level nesting drift silently kills
  the styling.
- **`:where()` zero specificity + `@layer` ordering.** DaisyUI zeroes its
  specificity so *you* can override it, but that also means Tailwind **preflight**
  (bare `button`/`ul`/`li`/`summary` resets — normal type-selector specificity,
  unlayered) beats DaisyUI's `:where(...)`-zeroed rules. Example: preflight's
  `summary { display: list-item }` clobbers daisyUI's `summary { display: grid }`.
- **Semantic classes encode layout, not just look.** `dropdown` sets
  `display: inline-block`, `menu` sets `display: flex`, `navbar` sets
  `display: flex` + `min-height`. Two classes on one element collide over the
  same `display` property, and the layout *invariant* is coupled to the skin.
- **State as CSS pseudo-classes.** Dropdown/menu open state is driven by
  `:focus-within`, `details[open]`, or `.dropdown-open` — CSS state, not app
  state. This fights Leptos's model where a `RwSignal<bool>` is the single source
  of truth (and is exactly why the native `popover`/`popovertarget` open/close
  went stale).
- **Stringly-typed variants.** `btn btn-ghost btn-square` is a string of
  modifier tokens; a typo silently no-ops.

What daisyUI does **idiomatically**: token-driven theming
(`--color-base-content`, `--color-neutral`, …) and composable visual utilities.
Keep those.

### 9.3 The self-containment contract (the rule going forward)

For any **reusable `#[component]`**, hold these invariants so it renders
identically in any screen:

1. **Namespace the root.** Give the component a single prefix class (e.g.
   `.flag-dropdown`) and style its *structural* properties behind that class.
2. **Own the structural spine.** Re-assert `position`, `display`, flex, sizing,
   and `z-index` **unlayered and non-`:where()`** (specificity `> 0`) in the shell
   stylesheet. Unlayered and non-`:where()` beats daisyUI's layered/`:where()`
   rules *and* global resets.
3. **Never style bare global elements inside a component.** If the component
   renders a `<button>`, style `.flag-dropdown__item`, not `button`. This makes
   it immune to `sanitize.css`/preflight and to other screens' element rules.
4. **Drives open/close (and any state) from a Leptos signal**, not from CSS
   pseudo-classes (`:focus-within`) or the native `popover` toggle. Bind
   visibility via `style:display` (or `class:` on a namespaced modifier).
5. **Floating UI owns its positioned ancestor.** A dropdown/menu/tooltip must
   (a) be anchored to its own `position: relative` wrapper that carries that
   invariant explicitly (see the `.flag-dropdown` wrapper), or (b) live in the
   top layer. Never rely on a daisyUI wrapper to supply `position: relative`.
6. **Use Tailwind utilities for values, namespaced classes for structure.**
   `p-2`, `rounded-box`, `flex` are fine as utilities; the *structural
   invariants* a component must never lose live only in its namespaced class.

**Litmus test:** *"If I drop this markup into an unrelated screen that has
`sanitize.css` + Tailwind preflight + a flex row + `overflow: hidden`, does its
layout still hold?"* If the answer depends on the answer, the component is not
self-contained yet.

### 9.4 Reference

The canonical example is the owned primitives in `client/shells/leptos/src/ui.rs`
(`UiButton` with `tw_merge!`, `UiDialog` with signal-driven `showModal()`/`close()`,
`UiToggle` with `prop:checked`), plus `FlagDropdown` in
`client/shells/leptos/src/app.rs` (`position: relative` wrapper + `RwSignal<bool>`
open state + `position: absolute` menu). They own their structural invariants and
style via Tailwind utilities + theme tokens — the pattern to follow for any new
floating or layout-sensitive component.

### 9.5 Primitive ownership (post-daisyUI)

daisyUI has been removed — see `STYLING_MIGRATION.md`. The styling contract is
now:

- **No component library.** Every reusable visual unit is an owned Leptos
  primitive in `src/components/` (`UiButton`, `UiBadge`, `UiTable`, `UiToggle`,
  `UiTabs`/`UiTab`, `UiInput`, `UiSelect`, `UiCard`, `UiBreadcrumbs`,
  `UiSubmitButton`, `UiPagination`, `UiDialog`).
- **Tailwind utilities + theme tokens** (shadcn convention) do the visual styling
  (`bg-primary`, `text-primary-foreground`, `border-input`, `bg-muted`, …). The
  tokens live in `input.css` (`:root`/`.dark` → `@theme inline`).
- **`tw_merge!`** composes the primitive's base classes with a caller's `class`,
  resolving conflicts (later wins) instead of string concat.
- **Own behavior/state/structure.** Primitives own `<button type>`, `disabled`,
  ARIA, events, and any structural invariant (`position`, flex, `z-index`).
- **Page/layout CSS** (`thaw-*`, `card-browser__*`, `study-*`, `deck-options*`)
  lives in `input.css` under `@layer components`, not as unlayered author CSS.
- **Components own their styling via explicit utility classes.** No scoped
  `<style>`, no CSS-in-Rust — matching rust-ui/shadcn. A component that renders a
  popover `ul`/`li`/`button` resets those elements itself with utilities
  (`list-none m-0`, item `p-*`, …) so ancestor reach-in cannot drift it.
- **App shell is utility-styled in place** (a signal-driven collapsible `<aside>`
  sidebar + a utility navbar), not a self-contained primitive — it sits once in
  `RootComponent`.

The §9.1–§9.3 guidance (inheritance vs. selector reach-in, why semantic bundles
caused drift, and the self-containment contract) still applies: the contract is
now *enforced by explicit utilities + `@layer`* rather than by a scoped
stylesheet.
