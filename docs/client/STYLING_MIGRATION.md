# Leptos shell styling migration — off daisyUI, onto owned Tailwind components

Status: **done**, on branch `style/daisyui-removal` (not yet merged to `main`).
Owned by the **shell agent**. See also `SHELLS.md` §9 and `CLIENT.md` §"Agent
ownership".

## 1. Goal

Replace daisyUI — whose semantic class bundles (`btn`, `menu`, `modal`, …) and
`@layer` + `:where()` internals have been the source of repeated,
container-dependent rendering bugs — with **owned Leptos primitives styled by
Tailwind utilities + CSS-variable theme tokens**, following the model of
[`rust-ui/leptos-ui`](https://github.com/rust-ui/leptos-ui) (a shadcn/ui-style
copy-paste registry for Leptos).

Concretely, after the migration:

- **No daisyUI** (`daisyui.mjs`, `@plugin "./daisyui.mjs"`, Fluent-token aliases,
  and every daisyUI class string) remains.
- Every visible unit is an owned `#[component]` in `ui.rs` whose styles are
  **Tailwind utility classes + theme tokens** (e.g. `bg-primary`,
  `text-primary-foreground`, `hover:bg-primary/90`).
- Call sites in `app.rs` use the primitives + utility classes, never a semantic
  bundle.
- Class merging uses `tw-merge` so a caller's `class` composes with the
  primitive's base classes without string-concat conflicts.

## 2. Why this eliminates the recurring bug class

daisyUI bundles *structure* into semantic classes (`display`, `position`,
padding) behind `:where()`-zeroed selectors inside `@layer daisyui.l1.l2.*`.
That makes its components lose the cascade to (a) Tailwind preflight / global
resets (unlayered beats layered) and (b) its own sub-layer ordering (`l1.l2.l3`
base beating `l1.l2` modifiers). The result has been "identical markup, different
rendering" depending on the mounted container, and broken open-states (e.g. the
`.modal[open]` dim losing to the base `transparent`).

Pure Tailwind utilities + tokens have no component layer and no `:where()`
zeroing, so this class of bug cannot occur by construction.

## 3. Theming — adopt `leptos-ui`'s defaults for now

We lift `leptos-ui`'s `:root` + `@theme inline` block verbatim (shadcn CSS-variable
convention), so utilities like `bg-primary` / `text-muted-foreground` / `border-ring`
resolve to real colors. We will **form our own design system later**; this is the
neutral grayscale default to unblock the migration.

Theme shape (from `leptos-ui` `style/tailwind.css`):

- `:root { --background, --foreground, --card, --card-foreground, --popover,
  --popover-foreground, --primary, --primary-foreground, --secondary,
  --secondary-foreground, --muted, --muted-foreground, --accent,
  --accent-foreground, --destructive, --border, --input, --ring, --success,
  --warning, … }` (all `oklch(...)`).
- `@theme inline { --color-primary: var(--primary); --color-primary-foreground:
  var(--primary-foreground); … --radius-*: …; }` so the `--color-*` namespace used
  by Tailwind utilities is populated.

## 4. Component mapping (leptos-ui → our primitive)

| leptos-ui registry | Our current primitive | After migration |
|---|---|---|
| `Button` (+ variant/size) | `UiButton` + `UiButtonAppearance` | utilities + tokens + `tw_merge` |
| `Badge` | `UiBadge` + `UiBadgeColor` | same |
| `Dialog` / `AlertDialog` | `UiDialog` | their utility dialog; drop hand-rolled `dialog.modal` CSS |
| `Input` | `UiInput` | utilities (`input-bordered` → `border-input` + `bg-background`) |
| `Select` (native) | `UiSelect` / `UiInput` | utilities |
| `Switch` | `UiToggle` | utilities |
| `Tabs` | `UiTabs` / `UiTab` | utilities |
| `Table` | `UiTable` | utilities + our `card-browser__table` wrapper |
| `Pagination` | `UiPagination` | utilities (drop `join`/`btn`) |
| `Breadcrumb` | `UiBreadcrumbs` | utilities |
| `Card` | `UiCard` | utilities |
| `DropdownMenu` / `Popover` | `FlagDropdown` + `StudyControls` dropdown | their popover/dropdown; remove `position:fixed`+`anchor-name` hand-roll |
| `Drawer` / `Sheet` | (raw `drawer`/`navbar` app shell) | their drawer/sheet |
| `Tooltip` | sidebar `is-drawer-close:tooltip` | their tooltip |
| — | `UiSubmitButton` | fold into `UiButton` (`type="submit"`) |

## 5. Boundaries / carve-outs

- **`tw-merge` must be added to `client/Cargo.toml` `[workspace.dependencies]`** —
  that is **core-agent** scope (see `CLIENT.md`). The shell agent flags this
  carve-out; it does not edit the workspace manifest itself.
- Everything else (`ui.rs`, `app.rs`, `index.html`, `input.css`, deleting
  `daisyui.mjs`, per-shell `Cargo.toml` dependency entries) is **shell-agent**
  scope.

## 6. Phases (incremental; each keeps `main` building)

Order is chosen so `main` stays green at every step (daisyUI remains loaded while
primitives migrate off it, then is deleted last).

1. **Phase 0 — tokens + `tw-merge` (foundation).**
   - Core agent adds `tw-merge` to `[workspace.dependencies]`; shell agent wires
     it into `leptos_shell/Cargo.toml`.
   - Add the `:root` + `@theme inline` block (shadcn tokens) to `input.css`
     **alongside** daisyUI (both loaded), so `bg-primary` etc. become valid.
2. **Phase 1 — primitives styling swap (the bulk).**
   - Rewrite each `Ui*` class string in `ui.rs` to utilities + tokens via
     `tw_merge`. Signatures/behavior unchanged, so `app.rs` call sites keep
     compiling. Migrate: `UiButton`, `UiBadge`, `UiCard`, `UiInput`, `UiSelect`,
     `UiToggle`, `UiTabs`/`UiTab`, `UiTable`, `UiPagination`, `UiBreadcrumbs`,
     `UiDialog`, `UiSubmitButton`.
3. **Phase 2 — floating UI.** Replace `FlagDropdown` + `StudyControls` dropdowns'
   `position: fixed` + `anchor-name` approach with a utility-based popover/dropdown.
4. **Phase 3 — app shell.** Replace raw `drawer`/`navbar`/tooltip with utility
   equivalents; delete the `.app-sidebar`/`.drawer*`/`.card-browser*` structural CSS
   that's no longer needed (keeping genuinely structural shell rules).
5. **Phase 4 — delete daisyUI.** Remove `daisyui.mjs`, the `@plugin "./daisyui.mjs"`
   import, the Fluent-token aliases, and any residual daisyUI class string.
   Confirm `cargo build` + `trunk build` clean, zero daisyUI references.

## 7. Proof-of-concept first

Before the full sweep, validate the foundation on **`UiButton` only**: add
`tw-merge` + tokens, rewrite `UiButton` to utilities + `tw_merge`, keep daisyUI
loaded for the other primitives, build, and confirm no visual/behavioral
regression. Then repeat for the rest.

## 8. Done

- [x] `cargo build` green in the client workspace (native + wasm).
- [x] `cargo test` green in the client workspace (shared tests unaffected).
- [x] `trunk build` succeeds (daisyUI plugin removed, shadcn theme active).
- [x] `daisyui.mjs` deleted; no daisyUI class string remains in markup.
- [x] `ui.rs` primitives + `app.rs` call sites restyled with Tailwind utilities
      + theme tokens via `tw_merge`.
- [x] `SHELLS.md` §9 updated to reflect the post-daisyUI contract (§9.4/§9.5
      rewritten; daisyUI-consumption tables §9.6/§9.7 removed).

Note: the neutral grayscale palette is the intentional `leptos-ui` default;
visual tuning toward our own design system is a separate follow-up.

---

## 9. Phase 2 — adopt `rust-ui/leptos-ui` as the component source

We are duplicating `leptos-ui`'s components already (same Tailwind utilities,
same shadcn tokens, same `tw_merge`). Rather than hand-author each `Ui*` a second
time, adopt their registry as the **copy-and-adapt source**: vendor their
component bodies under our existing `Ui*` names/signatures so `app.rs` call sites
keep compiling.

Decisions (locked):
- **Icons** — keep `lepticons` (already category-gated for WASM size; same Lucide
  glyphs). Do **not** add their `icons` crate.
- **Variants** — adopt their `variants!` macro → `tw_merge`'s `TwClass`/`TwVariant`
  derives (requires the `tw_merge` `variant` feature).
- **Branch** — continue on `style/daisyui-removal` (no new branch).

### 9.1 New dependency surface (core-agent carve-outs)

The shell agent cannot edit `client/Cargo.toml` `[workspace.dependencies]`; these
are flagged to the core agent:

1. **`tw_merge` `variant` feature** — change `tw_merge = "0.1"` to
   `tw_merge = { version = "0.1", features = ["variant"] }`. This pulls in
   `tw_merge_variants` (the `TwClass`/`TwVariant` derives).
2. **`paste`** — `paste = "1"` (their `variants!` macro uses `paste!` for
   identifier pasting).
3. (No `icons` crate — we keep `lepticons`.)

### 9.2 Vendor the `variants!` macro

Copy `crates/leptos_ui/src/variants.rs` (their `leptos_ui` crate's macro) into our
`ui.rs` (or a small local `variants` module). We take **only** the `variants!`
macro, re-exporting `tw_merge::*` + `paste`, **not** the whole `leptos_ui` crate
(we're not adopting their `icons`/`ui-cli`/`leptos_router` surface). Note their
macro's `support_href` branch pulls `leptos_router` — we omit that branch since
we use our own hash routing, not `leptos_router`.

### 9.3 Vendor components, re-exposed as our `Ui*` API

For each primitive, copy the registry body's `variants! { … }` block + thin
wrapper, then re-expose under our existing signature (or adjust `app.rs` call
sites where the signature diverges — e.g. their `Button` takes
`variant`/`size`/`class` as `Signal`s + optional `href`, whereas ours takes an
`appearance` enum + `on_click`).

| rust-ui component | → our `Ui*` | notes |
|---|---|---|
| `Button` | `UiButton` | variant+size enums (Default/Secondary/Destructive/Outline/Ghost/Link; Default/Sm/Lg/Icon) |
| `Badge` | `UiBadge` | variant = our `UiBadgeColor` (Default/Secondary/Destructive/Success/Warning/Outline) |
| `Dialog`/`AlertDialog` | `UiDialog` | signal-driven `showModal()`/`close()` |
| `Input` | `UiInput` | |
| `Select` (native) | `UiSelect` | |
| `Switch` | `UiToggle` | |
| `Tabs` | `UiTabs`/`UiTab` | |
| `Table` | `UiTable` | |
| `Breadcrumb` | `UiBreadcrumbs` | |
| `Card` | `UiCard` | |
| `Pagination` | `UiPagination` | |
| `DropdownMenu`/`Popover` | `FlagDropdown` + `StudyControls` menu | |

### 9.4 Theme

Replace our trimmed `input.css` `:root`/`.dark`/`@theme inline` with their verbatim
block (plus any tokens our call sites actually use), so `bg-primary/90` etc.
resolve identically to their docs.

### 9.5 Order (proof-of-concept first)

1. Core agent adds `tw_merge` `variant` feature + `paste` (carve-out 9.1).
2. Vendor `variants!` macro; keep `lepticons`.
3. Vendor `Button` → `UiButton` as the POC (prove `variants!` + `tw_merge`
   `variant` compiles in our workspace).
4. Repeat for the rest (badge, dialog, input, select, switch, tabs, table, card,
   breadcrumb, pagination, dropdown).
5. Swap to their verbatim theme; `cargo build`/`trunk build` green; update
   `SHELLS.md` to name `rust-ui/leptos-ui` as the adopted reference.

Open question deferred: whether to keep our `UiButtonAppearance` enum as a thin
compatibility layer over their `variants!` enums (recommended, so `app.rs` churn
is minimal) vs. migrating `app.rs` to the new `variant`/`size` signal API directly.

### 9.6 Status (Phase 2)

- [x] Dependencies: `tw_merge` `variant` feature + `paste` (core-agent `c5c26d6`).
- [x] `variants!` macro vendored (`src/variants.rs`).
- [x] `UiButton` → `variants!` (`UiButtonClass`/`UiButtonVariant`/`UiButtonSize`),
      with `UiButtonAppearance` as a compatibility enum.
- [x] `UiBadge` → `variants!` (`UiBadgeClass`/`UiBadgeVariant`), with
      `UiBadgeColor` as a compatibility enum.
- [x] Theme: our trimmed `input.css` already provides every token our components
      use (`--primary`/`--secondary`/`--muted`/`--accent`/`--destructive`/
      `--success`/`--border`/`--input`/`--ring` + `-foreground`), matching
      `leptos-ui`'s values — no verbatim-swap needed.
- [x] Floating UI: `FlagDropdown` + `StudyControls` menu now use CSS anchor
      positioning (`position: fixed` + `position-anchor` + `position-area`), the
      idiomatic `leptos-ui` `Popover` approach, so they're top-layer and immune to
      `overflow` clipping.

**Finding:** `variants!`/`TwClass`/`TwVariant` is only a win for components with a
*variant matrix* (button appearance/size, badge colour). The single-skin primitives
(`UiInput`, `UiSelect`, `UiToggle`, `UiTabs`/`UiTab`, `UiTable`, `UiCard`,
`UiBreadcrumbs`, `UiPagination`, `UiDialog`) are already correctly utility-styled
via `tw_merge!` and gain nothing from `variants!` — wrapping them would be the
over-engineering this section warns against. They are therefore left as-is.

## 10. Phase 3 — adopt rust-ui components verbatim (in progress)

Goal: replace our hand-rolled `Ui*` primitives with rust-ui/leptos-ui's component
source **unmodified** (no `Ui` prefix; `Button`, `Badge`, `Card`, `Input`,
`Switch`, `Tabs`, `Table`, `Breadcrumb`, `Pagination`, `Select`, `Dialog`), and
record the app-specific things rust-ui has no equivalent for so we can
idiomatize them later.

### 10.1 Dependency reality (why "verbatim" is bounded)

rust-ui's components are not self-contained: they import from the `leptos_ui`
crate (`variants!` and `clx!` macros), the `icons` crate (`ChevronRight`,
`ChevronLeft`, `Ellipsis`, …), `strum` (Input's `InputType`), and — for
`Pagination` — `leptos_router` (`use_navigate`). Full verbatim adoption therefore
requires pulling in `leptos_ui` + `icons` + `strum` (+ `leptos_router` for
pagination), which is a real dependency decision, not a rename.

Concretely, the verdict **per component** (checked against the rust-ui source at
`app_crates/registry/src/ui/*` and `public/registry/styles/default/*.md`):

| rust-ui component | Depends on | Verbatim-able now? |
|---|---|---|
| `Button` | `leptos_ui::variants` (already vendored) | ✅ — restate `variants!` with rust-ui's exact class strings |
| `Badge` | `leptos_ui::variants` (already vendored) | ✅ — same |
| `Tabs`/`TabsList`/`TabsTrigger`/`TabsContent` | **only** `leptos` + `tw_merge` | ✅ fully verbatim |
| `Switch` | `leptos_ui::clx` (trivial) | ✅ — inline the `clx!` wrapper class |
| `Card` (+`CardHeader`/… | `leptos_ui::clx` | ⚠️ needs `clx!` macro |
| `Table` family | `leptos_ui::clx` | ⚠️ needs `clx!` macro |
| `Breadcrumb` family | `leptos_ui::clx` + `icons` | ⚠️ needs deps |
| `Input` | `strum` | ⚠️ needs `strum` |
| `Select` / `SelectNative` | `leptos_ui` + JS portal runtime | ❌ JS layer — deferred (see §10.3) |
| `Dialog` / `DropdownMenu` | JS portal + `use_lock_body_scroll` | ❌ JS layer — deferred |
| `Pagination` | `leptos_router` | ❌ router-based; our Crux hash-router differs |

### 10.2 Decision (Option 2, as agreed)

Adopt the **pure-Leptos / already-vendored** subset verbatim now, and keep the
floating/modal layer (`Select`, `Dialog`, the study ⋮ `DropdownMenu`) on our
CSS-anchor / native-`<dialog>` approach, *named plainly* so call sites read
`Select`/`Dialog`/`DropdownMenu` rather than `Ui*`. The JS-portal layer is a
separate later decision.

### 10.3 Deferred (decide later): JS floating layer

rust-ui's `Select` popover, `Dialog`, `DropdownMenu` don't use CSS anchor —
they use a `Goggles` portal + `ScrollLock`/`use_lock_body_scroll` + a `<script>`
fragment shipped in `public/app_components/*.js`. That is out of scope for this
phase.

### 10.4 Not satisfiable by rust-ui (record for later idiomatization)

These are Anjuman-specific and have **no** rust-ui equivalent; they remain ours.
Recorded so we can later shape them to match rust-ui's conventions (data-*
attributes, `clx!`-style one-component-per-file, `variants!` where a matrix
exists):

1. **Study card surface** — sandboxed-iframe card + reveal/answer buttons.
2. **Deck tree** — tabular deck list with aligned New/Learn/Due count columns
   (rust-ui has `Collapsible` but no aligned-count tree).
3. **Filter sidebar tree** — the Cards/Notes browser filter tree where a parent
   deck is *also* a selectable filter, with flag-colour swatch leaves.
4. **Card/Note inspector** — the right-hand action/details pane (a `Card` +
   `dl` composition we own).
5. **`flag_swatch`** — the coloured-dot + label Anki-flag swatch.
6. **Collapsible sidebar app shell** — our icon-rail ↔ width-collapse `<aside>`;
   rust-ui's `Drawer`/`Sheet`/`SideNav` are overlay/route nav, not width-animated
   rails.

### 10.5 One necessary divergence: controlled vs. internal state

rust-ui's `Tabs` *owns its selection* internally (`TabsCtx { selected:
RwSignal<String> }`, `default_value=`, `TabsTrigger(value=)` mutates it). We are a
**Crux** shell, so the active tab must mirror the core's `ViewModel.route`
unidirectionally — not a separate local `RwSignal` that forks and goes stale.

Therefore `Tabs`/`TabsList`/`TabsContent` are adopted **verbatim** (they're
presentational), but `TabsTrigger` is a **controlled** variant: it takes
`value`, an `active: Signal<bool>` derived from the route, and `on_click`, and
renders rust-ui's exact class strings without `expect_context`-mutating internal
state. This is the one place where "default code unmodified" yields to the
unidirectional-dataflow invariant that has been non-negotiable through this
whole shell.

### 10.6 State + naming divergences (revisit later)

| Component | Divergence | Resolution to revisit |
|---|---|---|
| `Switch` | rust-ui owns its state (`RwSignal`) and toggles internally; ours is controlled (`checked`/`on_toggle`). | Cardinal unidirectional-dataflow — keep. |
| `Tabs` | rust-ui owns selection internally (`TabsCtx`); ours is controlled (`active: Signal<bool>` + `on_click`). | Cardinal — keep (§10.5). |
| `Button` | rust-ui variant names (`Default`/`Ghost`/`Secondary`/…) vs our `Primary`/`Subtle`/`Danger`/`Neutral`/`NeutralSoft`. | Rename our call sites to rust-ui's vocabulary (or accept a permanent thin `ButtonAppearance` alias). |
| `Badge` | rust-ui variant names vs our `Brand`/`Danger`/`Success`/`Informative`. | Same — reconcile names. |
| `TableRow` | rust-ui's is presentational-only; ours adds controlled `selected`/`on_click` (selectable browser rows). | Cardinal shell interactivity — keep; class strings are verbatim (`data-state` drives the selection class). |
| `Card` / `Badge` / `Button` | `Size`/`Variant` matrix entries unused by callers. | Remove the module `#![allow(dead_code)]` as callers adopt the full matrix. |

The `Button`/`Badge` class **strings** are rust-ui's verbatim already; only the
call-site enum names are a naming gap. Both `button.rs` and `badge.rs` (and `card.rs`)
carry a module-level `#![allow(dead_code)]` because the verbatim variant/size matrix
includes entries our callers don't use yet. Remove that allow as callers adopt
the full matrix.

### 10.7 Status (verbatim adoption, in progress)

`Ui` prefixes dropped; leaf components adopted verbatim (class strings only) or
renamed plain pending the JS-layer decision:

| Component | Status |
|---|---|
| `Button`, `Badge` | ✅ verbatim `variants!` (naming gap §10.6) |
| `Switch`, `Tabs`/`TabsList`/`TabsTrigger` | ✅ verbatim, controlled (§10.5) |
| `Card`, `Input`, `Breadcrumb`, `Table` family | ✅ verbatim class strings (`TableRow` controlled, §10.6) |
| `Select`, `Dialog`, `Pagination` | ⚠️ renamed plain, **not** verbatim — rust-ui's are JS-portal (§10.3) / `leptos_router` (§10.1) |
| `CardHeader`/`Title`/`Content`/… , `BreadcrumbSeparator`/`Page`/… | not yet — nothing consumes them; add verbatim when needed (breadcrumb separator pulls `icons`) |

`UiSubmitButton` was folded into `Button` (`type="submit"`), per §4.
