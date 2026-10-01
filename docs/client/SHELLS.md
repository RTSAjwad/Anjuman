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

- **The current screen/route is `ViewModel` state, not shell state.** Which deck
  is open is `ViewModel.selected_deck` (`Option<DeckSummary>`; `None` ⇒ the deck
  list). Which panel of that deck is showing is `ViewModel.selected_deck_view`
  (`DeckView::{Study, Options}`, a core-local `Facet` enum). A shell renders the
  list when `selected_deck` is `None`, otherwise the panel named by
  `selected_deck_view`. It must **not** introduce its own `active_view`/`route`
  signal to shadow these fields.

- **Navigation is an `Event`, and the core chains the follow-up fetch.** Opening
  a deck's Study view is `Event::OpenDeck { deck_id }`; opening its Options view
  is `Event::OpenDeckOptions { deck_id }`. The core records the selection *and*
  auto-chains `StartStudy` (for `OpenDeck`) or `DeckOptionsRequested` (for
  `OpenDeckOptions`) and returns a `Render`. A shell therefore **forwards the open
  event only** — it does **not** fire `StartStudy`/`DeckOptionsRequested` itself,
  or the fetch would happen twice.

- **Back-navigation resets the route in the core.** Leaving a deck is
  `Event::CloseDeck` (or the logout flow): the core clears `selected_deck` and
  resets `selected_deck_view` to `Study`. The shell does not need to reset
  anything — the next `OpenDeck`/`OpenDeckOptions` lands on the requested panel.

- **Transient UI state stays shell-local.** While the *route* belongs to the
  core, purely presentational state that no behaviour depends on (e.g. the
  Leptos shell's card `revealed` flag, an expanded/collapsed tree) may live in
  the shell. The test: if another shell can't see it and no `Event`/`ViewModel`
  field needs it, it may be shell-local; anything that affects *what the user
  can do* (which panel, which deck) is core-owned.

This is the Crux-idiomatic answer to routing: URLs/`leptos_router` are web-only
and would duplicate core state. The five shells share one routing model carried
by the `ViewModel` + `Event`s, not per-shell URL frameworks.

## 4. The effect loop (canonical shape)

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

## 5. Implementing each capability

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

## 6. Where a shell's work is *specified*

A shell agent does **not** read the user stories and invent behaviour — it
satisfies the **shell contract** and the **coverage matrix**:

- **`ROADMAP.md`** ([`docs/process/ROADMAP.md`](../process/ROADMAP.md)) — each story ends with a `Shell contract` checklist (render X,
  forward Y, execute Z). That is the shell's work order.
- **`SCREENS_SUPPORT.md`** ([`docs/support/screens.md`](../support/screens.md)) — the "Shell coverage" matrix is the parity
  ledger; tick your shell's column as you complete a contract.

If the contract is silent on something a shell needs (an ambiguous prop, a
missing event), ask — don't invent a convention. The contract is the interface
between the core agent and the shell agent.

## 7. Per-shell gotchas (live here, not in the shared docs)

- **Leptos** — CSR-only single binary (no `lib` target). Wraps the tree in thaw's
  `<ConfigProvider>`; prefers plain `<div>` for layout (nesting thaw components
  that return `impl IntoView` inside `Space`/`Card` triggers an `IntoFragment`
  error). thaw's `Input` is driven by a `Model<String>` (an `RwSignal` via its
  `value` prop), not an `on_change` callback. Uses `gloo-net` + `gloo-storage`.
- **SwiftUI / WinUI / Compose** — FFI shells; see [`CLIENT.md`](./CLIENT.md) §BoltFFI for
  the binding + typegen flow (`boltffi generate <lang>`).

Add a section here for each shell as it is implemented.
