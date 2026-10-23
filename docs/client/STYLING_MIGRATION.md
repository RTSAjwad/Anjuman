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
