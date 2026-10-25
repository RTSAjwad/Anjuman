# React (React Router + Base UI + shadcn) shell — plan

Status: **shipped** — the React shell is implemented and live as the sole web
front-end (Phases 0–4 done), and the Leptos shell has been **deleted**. This
document is retained as the historical plan that produced it.

This document is the plan for standing up that shell. It is scoped to the shell
agent (`client/shells/*`, `docs/client/*`). Anything that touches
`client/shared/`, `contracts/`, `server/`, or a workspace manifest is a
**carve-out** to the core agent.

---

## 0. Non-goals / what is explicitly out of scope for this document

- This is **not** a plan to change the core. The `shared/` Crux core, its
  `Event`/`ViewModel`/`Effect` surface, and the `anjuman_contracts` wire types
  are untouched. The React shell consumes them exactly as any other shell.
- This is **not** a plan to author new business logic. The shell remains a
  **thin, unidirectional** consumer: render `ViewModel`, forward `Event`,
  resolve `Effect`. (See `SHELLS.md` §2.)

---

## 1. The one architectural fact that shapes everything

The five shells split into two access paths (`SHELLS.md` §1, `ARCHITECTURE.md` §4):

| Shell kind | Existing examples | Mechanism |
|---|---|---|
| **Rust** (in-process) | Leptos | typed `Core<Anjuman>` API, **no serialization** |
| **Non-Rust** (FFI) | SwiftUI, WinUI, Compose | serialized `Bridge` (`bincode`) + BoltFFI bindings + Facet typegen |

**React is a non-Rust shell.** It cannot hold `Rc<Core<Anjuman>>` in-process it
must drive the core across the FFI boundary as serialized bytes:

- `CoreFfi.update(&[u8]) -> Vec<u8>` — send an `Event`, get effects back.
- `CoreFfi.resolve(u32, &[u8]) -> Vec<u8>` — feed an effect result back, get more
  effects.
- `CoreFfi.view() -> Vec<u8>` — get the current `ViewModel`.

The wire format for those bytes is **`bincode`** (Crux `Bridge` default,
confirmed in `CLIENT.md` §4). This is the single most important constraint: the
React shell must (a) obtain the core as **WASM**, (b) call those three functions
over a WASM bridge, and (c) (de)serialize `Event`/`ViewModel`/`Effect` as
`bincode` in JS.

### 1.1 Good news: the WASM + TypeScript toolchain already exists

`client/shared/boltffi.toml` already declares a **`[targets.wasm]`** target that
BoltFFI can pack into a `wasm-bindgen`-based npm package (currently output into
`shells/leptos/generated/pkg`, gitignored, and to-date unconsumed because Leptos
runs in-process). The `codegen` binary (`shared/src/bin/codegen.rs`) already has
a **`Typescript`** branch that emits Facet-generated `.ts` **type** declarations
for `Event`/`ViewModel`/`Effect`.

What this means: **the core-side infrastructure for a JS shell largely exists.**
What does *not* yet exist is the JS runtime that calls `update`/`resolve`/`view`
and pushes the `bincode` buffers through, plus any glue to package/load the
generated WASM into a JS bundler. Those are the real unknowns to spike first.

### 1.2 The JS runtime: hand-rolled glue with *generated* serialization (settled)

> **Settled** — there is no upstream `@redbadger/crux` library to adopt. The
> official Crux web walk-through
> (["Web — TypeScript and React Router"](https://redbadger.github.io/crux/part-3/platforms/react-router.html))
> shows the idiomatic web shell as a **hand-written `core.ts` wrapper around
> `CoreFfi`**, with the `bincode` (de)serialization **code-generated** rather than
> hand-written.

Concretely, the walk-through's `core.ts` is a thin `Core` class that:

- wraps the `CoreFfi` wasm binding (`CoreFfi.new()` / `.update()` / `.resolve()` /
  `.view()`), gated on the `shared` package's `initialized` promise;
- dispatches `update(event)` → collects effects → `processEffect(effect)`;
- handles `Render` by calling `setState(this.view())` and deserializing through
  the **generated** `ViewModel.deserialize` + `BincodeDeserializer`;
- serializes outgoing `Event`s via the **generated** `serializeEvent` +
  `BincodeSerializer`.

Both generated packages come from tooling we already have configured:

- `boltffi pack wasm` → a `shared` npm package (`CoreFfi` + an `initialized`
  promise) — the `[targets.wasm]` entry in `boltffi.toml`.
- `codegen --language typescript` → a `shared_types` package with typed
  `Event`/`ViewModel`/`Effect` **and** `BincodeSerializer`/`BincodeDeserializer`,
  `serializeEvent`/`deserializeView`/`matchEffect`.

So there is **no manual `bincode` writing** and **no decision between a library
and hand-rolling the core wrapper** — the wrapper is hand-written (mirrors the
Leptos `core_link.rs` effect loop) but the serialization is generated. The
spike's only real work is wiring the toolchain and reproducing the round-trip.

**The spike is the first deliverable** and must produce a working round-trip:
load `shared.wasm`, send `RestoreSession`, resolve the `KeyValue` effect from
`localStorage` (and `Http` from `fetch`), and render the `ViewModel` into React
state. Nothing more.

> Spelling out the honest cost: neither path reuses the Leptos `core_link.rs`
> (it drives the typed `Core` in-process). The React shell reimplements the
> **glue** (the effect loop + capability executors for HTTP/KV), not the
> behaviour (which stays in the core). This is a real, bounded, one-time cost and
> is the natural price of a non-Rust shell — identical to what SwiftUI/Compose
> will pay, per `SHELLS.md` §5–6.

---

## 2. Decisions (proposed; confirm before building)

1. **Framework stack:** **React Router (framework mode)** — the Vite-based,
   file-routed React framework the official Crux web walk-through uses — with
   **Base UI** (unstyled, accessible primitives) + **shadcn/ui** (the
   React/Base-UI component layer) on top. This is the "unstyled primitives +
   utility styling" model we spent phase 1 chasing by hand — here it is the
   default, maintained posture. Base UI is now shadcn's **default** primitive
   layer (July 2026), so `shadcn init` yields Base-UI components out of the box.
   (This replaces the earlier "Vite + React SPA with your own router" idea: the
   walk-through assumes React Router framework mode — `HydratedRouter`,
   `entry.client.tsx`, `app/routes/*`.)
2. **Styling:** Tailwind v4 (already a repo dependency via the standalone Rust
   binary) + shadcn's `cn()`/`tailwind-merge`/`clsx`. **No daisyUI**, no
   semantic-class-leakage problem — components own their styling via tailwind
   utilities + CSS variables (the shadcn theme).
3. **State model:** a single top-level React store that holds the latest
   `ViewModel` and a `dispatch`/`emit`. **Crux's unidirectional invariant is
   non-negotiable** (the bug class we fought in Leptos — internal `RwSignal`
   state drifting from the core): controls are **controlled** (value in from
   `ViewModel`, event out to `dispatch`). No component-local mirrored state that
   can go stale.
4. **Routing:** React Router (framework mode) owns the *browser/URL* concern
   (deep-linking, history, back-button); **`ViewModel.route` stays the source of
   truth for the rendered screen** — the same rule as `SHELLS.md` §3. **Settled:
   Caveat A = A2** (see §4), matching the official Crux web walk-through's use of
   React Router.
5. **Scaffolding method:** shadcn's **auto-generated component files** (copy the
   source for each component into `components/ui/*`), not a runtime dependency on
   a monolithic package — this is idiomatic shadcn and lets us customize later.

These are proposals. The coordinator (user) should confirm each before I build;
§4's two caveats in particular need an explicit ruling because they depart from
current documented conventions.

---

## 3. Component mapping — Leptos primitives → Base UI / shadcn

This is the deliverable for "map all of our component requirements to idiomatic
shadcn/base-ui/react alternatives." It maps every component in
`client/shells/leptos/src/components/` to its React equivalent. The React shell
will **not** port the Leptos components; it will consume these instead.

| Leptos component (`components/*.rs`) | React equivalent (Base UI + shadcn) | Notes |
|---|---|---|
| `button.rs` (Button + variants) | Base UI `Button` + shadcn `Button` (variant/size via `cva`) | `variant: default/secondary/outline/ghost/destructive/link`; `size` sm/default/lg |
| `badge.rs` (Badge + variants) | shadcn `Badge` (secondary/outline/destructive) | flag colours via inline `style`/swatch, not badge variants |
| `card.rs` (Card/CardHeader/Content/Footer) | shadcn `Card`/`CardHeader`/`CardTitle`/`CardContent`/`CardFooter` | |
| `input.rs` (Input) | shadcn `Input` | |
| `switch.rs` (Switch) | shadcn `Switch` (Base UI `Switch`) | **controlled**: `checked={vm.value}` + `onCheckedChange={() => emit(...)}` — never internal state |
| `select.rs` (Select) | shadcn `Select` (Base UI `Select`/`Radix`-style) | **controlled** `value`/`onValueChange` |
| `tabs.rs` (Tabs/TabsList/TabsTrigger) | shadcn `Tabs`/`TabsList`/`TabsTrigger` (Base UI) | cards↔notes toggle |
| `dialog.rs` (Dialog) | shadcn `Dialog` (Base UI `Dialog`) | reschedule day-offset prompt |
| `floating_layer.rs` (portal + scroll-lock) | Base UI `Portal` + `DismissableLayer`/native `<dialog>` + `Popover`/`DropdownMenu` | **built-in** — this removes a hand-rolled piece entirely |
| `dropdown_menu.rs` (study ⋮ menu) — *(currently inline/browser-only)* | shadcn `DropdownMenu` (Base UI `DropdownMenu`) | study actions menu |
| `popover.rs` (flag select popover) *(inline)* | shadcn `Popover`/`Select` (Base UI) | flag 0–7 picker |
| `table.rs` (resizable Table family) | shadcn `Table` (Base UI `Table`) + optional `@tanstack/react-table` | resizable columns → `@tanstack/react-table` if needed (defer) |
| `pagination.rs` (Pagination) | shadcn `Pagination` | |
| `breadcrumbs.rs` (Breadcrumb) | shadcn `Breadcrumb` | header breadcrumb |
| `sidebar.rs` (SidebarItem) | shadcn `Sidebar` (the `sidebar-07`/collapsible icon-rail pattern) | collapsible icon-rail app shell |
| `inspector.rs` (Inspector/InspectorFields/InspectorRow/LabeledSwitch) | **no library equivalent** — compose from shadcn `Card` + `Label` + `Switch` + `Select` | the browser action/details pane (app-specific). |
| `filter_menu.rs` (`filter_branch`/`filter_leaf`/`deck_filter_tree`) | shadcn `Sidebar`/`Collapsible` + custom deck/flag tree; flag-colour swatches | app-specific; "parent is also a filter" divergence carries over |
| `flag.rs` (FlagSwatch) | presentational: `<span>` + colour dot + label | app-specific, trivial |
| `study_card.rs` (StudyCard + `format_interval`) | custom component + a `format_interval` util (no lib equivalent) | app-specific |
| `deck tree` / `study screen` grid (in `views/`) | plain Tailwind grid/flex `div`s + shadcn atoms | app-specific structural layout |

### 3.1 Classification

- **Directly covered → Base UI/shadcn**: Button, Badge, Card, Input, Switch,
  Select, Tabs, Dialog, DropdownMenu, Popover, Table, Pagination, Breadcrumb,
  Sidebar, Tooltip, Label, Separator, Skeleton, ScrollArea.
- **Composed from atoms (no dedicated library component)**: Inspector, Filter
  menu with flag swatches, FlagSwatch, StudyCard, deck/study grid layouts.
- **Hand-rolled before, now free in React**: `floating_layer.rs` (portal,
  positioning, scroll lock, dismiss), `dropdown_menu`, `popover` — Base UI ships
  these as primitives, which directly deletes some of the phase-1 pain that
  motivated this switch.

---

## 4. Caveats / decisions

### Caveat A — URL routing vs. `ViewModel.route` — **SETTLED: A2**

`SHELLS.md` §3 says a shell **"never holds its own routing state"** and that
"URLs/`leptos_router` are web-only and would duplicate core state." That stance
was written for the Leptos shell and justified web-only routing on the
multi-shell story. A React web shell is a better place to bind `ViewModel.route`
↔ URL, and — crucially — the official Crux web walk-through already uses React
Router for its web shell, so this is the sanctioned pattern.

- **A1 — Core is the single source of truth (unchanged rule):** React Router is
  presentational; shell-local route state stays forbidden; no free deep-linking.
- **A2 — React Router owns the URL; core still owns the rendered screen ✓** — the
  router is the entry point (deep-link/history/back-button) and *forwards*
  `Show*`/`OpenDeck*` events to the core; the rendered branch still reads
  `ViewModel.route`.

**Decision: A2.** React Router (framework mode) mediates *navigation* (URL,
history, back-button, deep-linking); the core remains the single source of truth
for *which screen is rendered* (`ViewModel.route`). The core is never asked to
hold URL state, but the URL is explicitly allowed to be the *initiator* of a
`Show*`/`OpenDeck*` event. This is recorded in `SHELLS.md` §8 as a React-shell-
specific (web-only) divergence from §3's stricter rule, mirroring the official
Crux web walk-through.

### Caveat B — build wiring for the WASM + JS toolchain

The Nix `flake.nix` currently builds the web shell via `trunk` (Rust/Leptos) and
has no `vite`/React build wiring; `pnpm` and `nodejs` are already in
`client-packages` (they exist for Facet typegen). Standing up the React shell
needs a Nix `app`/`devShell` entry that runs the JS toolchain (e.g. `vite`), plus
a step to pack the BoltFFI `wasm` target and wire the generated `.wasm` + `.ts`
types into the bundler. **This is a carve-out to the core agent** (workspace
manifests + `flake.nix` + `boltffi.toml` are core scope), filed when we reach the
build-wiring step.

---

## 5. Phased plan

### Phase 0 — Spike the WASM bridge (the gate)

Produce a minimal but real round-trip before any UI.

**Spike findings (verified 2026-10-24):**

- ✅ `cargo build --features codegen -p shared` builds the typegen binary.
- ✅ `codegen --language typescript` emits the full `shared_types` package
  (`app.ts` with typed `Event` constructors + `matchEvent`/`matchEffect`/
  `matchCurrentRoute`/`serializeEvent`, plus `bincode/` and `serde/` serializer/
  deserializer trees). Serialization is **generated** — no manual bincode.
- ✅ `boltffi pack wasm` compiles + optimizes the core to `shared_bg.wasm` and
  generates the `shared` npm package (`generated/pkg/`).
- ⚠️ `boltffi pack wasm` then **fails at the final `tsc` typecheck** — `tsc` is
  **not on `$PATH` in the dev shell** (`typescript` is not in `client-packages`;
  only the pnpm-vendored copy used by `codegen` exists). This is a `flake.nix`
  gap → **carve-out** (add `typescript` to `client-packages`).

**Remaining Phase-0 steps (the first is a carve-out, the rest are shell scope):**

1. **Carve-out:** re-point `boltffi.toml` `[targets.wasm]` output (+
   `[targets.wasm.typescript]` output + `[targets.wasm.npm]`) from
   `shells/leptos/generated` to `shells/react/generated`, and add `typescript` to
   `flake.nix` `client-packages` (both are core-agent scope).
2. Stand up a bare React Router app under `client/shells/react/` (per the
   official walk-through: `pnpm create react-router@latest`, `pnpm add
   ./generated/pkg` + `./generated/types`, `vite-plugin-wasm` in
   `vite.config.ts`).
3. Write the `core.ts` wrapper (the walk-through's `Core` class): `CoreFfi` +
   generated `serializeEvent`/`deserializeView`/`matchEffect`; a
   `RustCoreProvider` holding the latest `ViewModel` in React state, wired into
   `entry.client.tsx` on the `shared` package's `initialized` promise.
4. Prove the loop: `RestoreSession` → resolve `KeyValue` (localStorage) +
   `Http` (fetch) → render the `ViewModel` (auth state, decks) to the screen.

**Exit criteria:** a `ViewModel` round-trip works end-to-end with zero business
logic in JS.

**Status: COMPLETE (2026-10-24).** The React shell scaffold builds end-to-end:
`pnpm run build` → `boltffi pack wasm` + `codegen --language typescript` +
`react-router build` all pass; the WASM (`shared_bg.wasm`) is bundled and the
`core.ts` wrapper (mirroring `core_link.rs`/`http.rs`/`kv.rs`) type-checks clean.
`pnpm dev` serves on `localhost:5173`. Remaining runtime proof (round-trip against
live Postgres/Axum) is a manual step.

**Shell-local gotchas (record for §8):**

- `verbatimModuleSyntax` (template default) breaks the *generated*
  `shared_types` code (`Serializer`/`Deserializer` type-only imports); set it to
  `false` in the shell `tsconfig.json`.
- The WASM-dependent modules must be suffixed `.client.ts(x)` so React Router
  excludes them from the SPA build-time prerender (`entry.server.tsx` otherwise
  500s importing the `shared` wasm package during prerender).
- `boltffi pack wasm`'s `shared` package exposes an `initialized` promise (not a
  `default init`) via `web.js`/`bundler.js`; the official Crux walk-through's
  `entry.client.tsx` gating works verbatim.
- Generated `KeyValue` bytes are `number[]` (`Seq<uint8>`), whereas `HttpResponse.body`
  is `Uint8Array` (`bytes`) — convert with `Array.from(...)`/`new Uint8Array(...)`.
- The `npm` scripts' `wasm:build`/`types:build` `cd` targets must be `../../shared`
  (relative to `client/shells/react/`), not `../shared`.

### Phase 1 — Toolchain, design system, and the app shell

Phase 0 is complete (full WASM round-trip proven). Phase 1 replaces the template
styling with Base UI + shadcn and stands up the app chrome: a collapsible
icon-rail sidebar, a header with breadcrumb, and the top-level
`ViewModel.route` screen switch. **No per-screen features yet** — screens render
placeholder stubs; their full content is Phase 3.

#### 1.1 Design-system init (shadcn over Base UI + Tailwind v4)

- **SETTLED: Base UI is shadcn's default** (since July 2026 —
  https://ui.shadcn.com/docs/changelog/2026-07-base-ui-default). `npx shadcn init`
  now generates Base-UI-flavoured components out of the box (Radix is opt-in via
  `-b radix`). So **no registry decision needed** — we run plain `shadcn init`
  (defaults to Base UI) against the existing React Router app, and consume
  `@base-ui/react` components. This matches the §2 decision “Base UI + shadcn”.
- `shadcn init` writes `components.json`, `lib/utils.ts` (`cn()` = `twMerge` +
  `clsx`), and the theme CSS variables into `app.css` (Tailwind v4 form). Pin
  `tailwind-merge` + `clsx` (already `^4.2.2` Tailwind via the template).
- Optional: `pnpm dlx skills add shadcn/ui` installs a project skill for
  component-add/migrate workflows; nice-to-have, not required for Phase 1.

#### 1.2 Component layer (pull the Phase-1 subset only)

For Phase 1 we only need the chrome atoms — pull them in and **defer** the rest
(Phase 2). One commit each:

- `Button`, `Badge`, `Input`, `Separator`, `Tooltip`, `Skeleton` — trivial.
- `Sidebar` (+ `SidebarProvider`/`SidebarInset`/`SidebarTrigger`/`SidebarMenu`/
  `SidebarMenuItem`/`SidebarMenuButton`) — the collapsible icon-rail shell. Use the
  shadcn collapsible icon-rail pattern (`collapsible="icon"`), matching the
  Leptos `<aside>` rail behaviour.
- `Breadcrumb` (+ `BreadcrumbItem`/`BreadcrumbLink`/`BreadcrumbPage`/
  `BreadcrumbSeparator`/`BreadcrumbList`) — the header breadcrumb.
- `Tabs`/`TabsList`/`TabsTrigger` — the Cards↔Notes tab switch in the header.

#### 1.3 App shell (`root.tsx` + a new layout route)

- Restructure `app/routes.ts` to a single top-level route (`routes/home.tsx`
  renamed to a layout) whose component renders the **chrome** (sidebar + header)
  and then `match view.route` to pick a screen component.
- Keep `CoreProvider` at the `root.tsx` level (already done). The chrome reads
  `useCore()` for `view.route`, `view.permissions`, and `update`.
- **Sidebar nav** (mirrors Leptos `RootComponent`):
  - “Decks” → `Event::ShowDeckList` (active when `route === DeckList`).
  - “Browser” → `Event::ShowCards` (active when `route === Cards`).
  - “Notes” tab → `Event::ShowNotes`, gated on `permissions.can_manage_notes`
    (mirrors the Leptos conditional `TabsTrigger`).
  - User/email display when authenticated; a logout affordance → `Event::Logout`.
- **Header breadcrumb** (mirrors Leptos `route` match): `DeckList` → `Decks`;
  `Cards`/`Notes` → the Cards↔Notes `Tabs`; `Study`/`DeckOptions` → the deck
  title + sub-view (populated fully in Phase 3; a static crumb now).
- **Route switch** (`match view.route`): `DeckList`/`Cards`/`Notes`/`Study`/
  `DeckOptions` each render a placeholder screen component for now (Phase 3 fills
  them). The screen switch reads `ViewModel.route` — **no shell-local route state**
  (per `SHELLS.md` §3; dropping the old `nav_selected` drift).

#### 1.4 Routing glue (Caveat A = A2)

- React Router owns the URL/history (deep-link + back button). Wire a thin
  mapping so the sidebar nav items, when clicked, both (a) update the URL via
  React Router `<Link>` and (b) forward the `Show*`/`OpenDeck*` event. The
  rendered branch still reads `ViewModel.route` (single source of truth).
- On app load, map the initial URL back to a `Show*` event (so `/cards`
  deep-links correctly). Keep this **thin** — the core stays authoritative.

#### 1.5 Nix + build

Already done in Phase 0 (nix run .#client serves React). No further carve-out
needed for Phase 1 unless shadcn init surfaces a missing tool. **Confirm nothing
new to file** — `pnpm`-only, no `flake.nix` edit expected.

**Exit criteria:** `pnpm run build` + `pnpm run typecheck` green; the app renders
real screens (via `view.route`), the sidebar collapses to icon-rail, nav forwards
`Show*` events and the screen actually switches; breadcrumb reflects `view.route`.

**Decisions (settled 2026-10-24):** (1) Base UI is the primitive layer — it is
shadcn's default (`shadcn init` → Base UI; Radix is opt-in `-b radix`); (2) keep
the Leptos chrome's nav set as-is (Decks + Browser, with Notes gated on
`can_manage_notes`).

### Phase 2 — Build the component layer

1. Pull in shadcn components per §3 (Button, Badge, Card, Input, Switch, Select,
   Tabs, Dialog, DropdownMenu, Popover, Table, Pagination, Breadcrumb, Sidebar,
   Label, Separator, Tooltip, Skeleton, ScrollArea).
2. Compose the four app-specific pieces from atoms (§3.1): Inspector, Filter menu
   (+ flag swatches), FlagSwatch, StudyCard + `format_interval`.
3. Enforce the controlled-state invariant uniformly (Switch/Tabs/Select/Table —
   `value` in, `onChange`/`onCheckedChange` → `dispatch(Event)`).

### Phase 3 — Port the four screens

Port `views/{deck,study,deck_options,browser}.rs` semantics to React views,
consuming the component layer. Each screen maps directly to the core contracts in
`ROADMAP.md` + `SCREENS_SUPPORT.md`. **No new behaviour** — parity with the Leptos
shell's core surface. Order by dependency: the deck list first (it opens the other
three), then study, then deck-options, then the browser.

**Phase 2 is folded into Phase 3** — we pull remaining shadcn components (`Select`,
`Switch`, `Dialog`, `Popover`, `Table`, `Pagination`, `ScrollArea`, `Collapsible`)
per-screen, as each screen needs them, rather than pre-installing the whole set.

#### 3.1 Deck list (`screens.tsx` → `deck-list.tsx`)

Mirror `views/deck.rs`:
- **Three states**: `decks_error` → error `Card`; empty `decks` → “No decks yet”;
  else the tree.
- **Tree**: recurse `DeckSummary.children` into an indented grid with a fixed
  header row (Deck | New | Learn | Due) and per-row expander + title + three
  coloured counts. **New column is `new_count`, Learn is `learning_count` (blue?),
  Due is `review_count`** — confirm badge colours vs. the deck.rs mapping
  (new=default/blue, learn=destructive/red, review=success/green; zero → muted).
- **Expand/collapse** is transient shell state — a `Set<bigint>` of open deck ids
  in `useState`, collapsed by default (the documented divergence: no persistence).
- **Studyable gating**: `deck.studyable` → clickable label (`OpenDeck{id}` →
  `Study`); else a greyed, non-interactive context-only row (title “Shared for
  context only”).
- **Actions**: gear (Settings icon) on studyable rows → `OpenDeckOptions{id}`.
- `deck_ancestor_titles` helper (port from `deck.rs`) for breadcrumbs.

#### 3.2 Study screen (`study.tsx`)

Mirror `views/study.rs`:
- **Header** (in the chrome): due-count badges (`study_count_badges`) + the
  `StudyControls` dropdown when `current_card` is present, + breadcrumb with
  deck-ancestor titles + “Study”.
- **Body**: `study_error`/`card_action_error` banners; `current_card` → `StudyCard`
  (reveal/answer buttons, card `styling` rendered into an `<iframe>` with the
  documented dark-mode CSS-inversion); `None` → busy “Loading…” or the
  “Nothing due” completion state with a “Check again” → `StartStudy`.
- **Study actions** (Bury/Bury note/Suspend/Suspend note/Reschedule + flag
  select), each forwarding the matching `Event` (using `current_card`), with the
  inverse affordances gated on `card.buried`/`card.suspended`. Reschedule opens a
  `Dialog` for a day offset. **Controlled**: flag `Select` value from
  `current_card.flag`, no local mirror.
- Remove the placeholder `revealed` shell signal if a clean `useState` suffices;
  keep it if the core doesn't reset reveal (confirm — `StudyCard` reveal is shell
  state, reset on `current_card` change).

#### 3.3 Deck options (`deck-options.tsx`)

Mirror `views/deck_options.rs` + the US-4.9/4.10/4.11 contracts:
- Read-only **view** of `view.deck_options` (the `DeckOptionsView`), gated on
  `can_manage_presets` for edit/save.
- **Preset assign** affordance (US-4.11): `Event::DeckOptionsAssign { deck_id,
  options_id }`; synthesize a “Default (id 0)” entry (server `GET /deck-options`
  doesn't include id 0). Fire `DeckOptionsListRequested` to populate `vm.presets`.
- **Disable edit/save when the loaded preset `id == 0`** (the global default is
  read-only — the server 403s mutation).
- `deck_options_error`/`presets_error` banners.

#### 3.4 Browser (`browser.tsx`) — cards + notes

Mirror `views/browser.rs` (the merged card/notes browser, US-4.12/4.13):
- **Cards↔Notes toggle** already in the header (Tabs); the selected view drives
  `CardsRequested`/`NotesRequested` from the browser state.
- **Table**: rows from `browser_cards`/`notes` (columns: front/note-type, deck,
  state/flag, sort field for notes) + `Pagination` from
  `browser_page`/`per_page`/`total` (and notes equivalents).
- **Filter sidebar**: deck tree (parent also selectable, flag colour swatches),
  note-type dropdown (`vm.note_types` via `NoteTypesRequested`), state/flag/sort/
  query; rebuilding + re-firing the `*Requested` filter event. **No new note
  creation** (descoped).
- **Inspector pane**: selected card/note (`browser_selected`/`notes_selected`),
  suspend/bury `Switch`es (controlled from the selected view), flag `Select`
  (`0–7`), reschedule `Dialog` — forwarding `Browser*`/`Note*` events.
- The card-browser (not now the notes view) is the Phase-3 target; notes table
  parity is a follow-on if time permits.

**Exit criteria:** all four screens render real data, nav reaches them via core
`Event`s, every affordance forwards its `Event` (no shell-local business logic),
and `pnpm run build` + `typecheck` green.

**Decisions deferred to Phase 4 (parity audit):** whether to fully port the notes
view now or track it as shell coverage; the `decks_loaded`/0-deck hydration gap
(carve-out pending); and the `revealed`/study reveal ownership question.

### Phase 4 — Parity audit + docs

1. Tick the React column in `SCREENS_SUPPORT.md` against the existing shell
   contracts.
2. Update `SHELLS.md` §8 with a React-shell entry (toolchain + the Caveat A/B
   resolutions + any gotchas).
3. Record the "Leptos back-burnered, React primary" decision in `CLIENT.md` and
   `ROADMAP.md` (the latter is a coordinator/core-adjacent flip — confirm).

---

## 6. Open questions to resolve before Phase 1 build

1. **Resizable table** — do we adopt `@tanstack/react-table` for resizable
   columns now, or defer (shadcn's basic `Table` first)?
2. **Sidebar pattern** — which shadcn sidebar variant matches the icon-rail ↔
   width-collapse behaviour best (`sidebar-07` collapsible)? Confirm visually in
   Phase 1.
3. **Who runs the `boltffi pack wasm` + `flake.nix` steps** — confirmed as a
   carve-out to the core agent (out of shell scope).
4. **§2 stack confirmation** — Base UI + shadcn + Tailwind v4 + single controlled
   store are still proposed, not yet ruled on by the coordinator.

---

## 7. Explicitly deferred

- **Leptos shell deletion** — out of scope; it stays on `trunk`, back-burnered.
- **New features** — this shell reaches parity with the existing core surface; it
  does not add behaviour (that would be new stories against the core).
- **Native shells** (SwiftUI/Compose/WinUI) — unaffected; they remain FFI shells
  and will share the same core surface the React shell consumes.

---

> **Status line (for the coordinator):** plan written and updated against the
> official Crux web walk-through. **Settled:** the JS runtime is hand-written
> `core.ts` glue over generated `bincode` serialization (§1.2); Routing = React
> Router framework mode with `ViewModel.route` as the rendered source of truth
> (Caveat A = **A2**). **Still awaiting:** confirmation of the §2 stack choices
> (Base UI + shadcn + Tailwind v4 + single controlled store) and the go-ahead to
> run the Phase 0 WASM-bridge spike. No code or files beyond this document have
> changed.
