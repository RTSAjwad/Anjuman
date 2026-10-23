# Leptos shell styling migration — off daisyUI, onto owned Tailwind components

Status: **planned** (not yet executed). Owned by the **shell agent**.
See also `SHELLS.md` §9 (the styling contract this supersedes/extends) and
`CLIENT.md` §"Agent ownership".

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

- `cargo build` + `cargo test` green in the client workspace.
- `trunk build` succeeds.
- No daisyUI references remain (`grep -r "daisyui\|btn\|modal\|menu\|badge\|drawer\|navbar\|breadcrumbs\|input-bordered\|select-bordered"` returns nothing relevant).
- `SHELLS.md` §9 updated to reflect the new (no-daisyUI) contract, and §9.6/§9.7
  boundary tables replaced with the post-migration state.
