# Carve-out (shell agent → core agent): finish Leptos-shell removal (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending
**Traceability:** `71f8763` ("remove Leptos shell + its docs", shell scope).

## Context

The React shell is now the sole web front-end, and the shell agent has already
deleted `client/shells/leptos/` + the Leptos-only docs (`STYLING_MIGRATION.md`)
and updated the shell-scope docs (`SHELLS.md`, `ARCHITECTURE.md`, `CLIENT.md`).

The **remaining Leptos references are all core-agent scope** per `AGENTS.md` §2
(workspace manifests, root `flake.nix`, repo-root `AGENTS.md`, `client/.gitignore`,
`client/shared/`). Please finish the removal:

## 1. `client/Cargo.toml` (workspace manifest)

- `[workspace] members` — remove `"shells/leptos"` (keep `"shared"`).
- `[workspace.dependencies]` — remove the now-unused web/Leptos pins that only the
  Leptos shell consumed: `leptos`, `console_error_panic_hook`, `lepticons`
  (category-gated lucide gate line), `tw_merge`, and `paste`. Also `gloo-net` is a
  direct dep of the deleted shell (it's in `shells/leptos/Cargo.toml`, not
  workspace — confirm and drop). Keep anything the React shell still needs (it
  consumes deps via `package.json`, not Cargo, so likely *none* of the Leptos block
  is needed).

## 2. `client/shared/Cargo.toml`

- The `[lib] crate-type` comment says `lib` (rlib) is kept "so other Rust crates
  (the Leptos shell) can depend on `shared` directly". With the Leptos shell gone,
  decide whether `crate-type = ["cdylib", "staticlib"]` now suffices (the React
  shell uses the WASM `cdylib` via BoltFFI, not the rlib). Update the comment
  accordingly. (If a future Rust shell — Libadwaita — would need the rlib again,
  note that and keep it; your call as core agent.)

## 3. `flake.nix`

`client-packages` currently lists Leptos tooling now unused by any shell:
- `trunk` — Leptos-only; remove.
- `tailwindcss_4` — was for Leptos's Tailwind; the React shell uses its own
  Tailwind via pnpm/Vite, not this binary. Remove (verify nothing else references
  it).
- `wasm-pack` — was for Leptos; `boltffi pack wasm` does not use it. Remove.
- `wasm-bindgen-cli` — confirm: `boltffi pack wasm` may still need it (keep if so).
- `binaryen`, `pnpm`, `nodejs`, `typescript`, `boltffi-cli` — keep (React shell).

Also update the process-compose `frontend` process description: it already runs
the React shell (`pnpm run dev` in `client/shells/react`), but the `nix run
.#client` app comment still says "serve the React shell" (correct); just confirm
no `trunk`/Leptos text remains. `nix run .#anjuman`'s `frontend` is already
React-only.

## 4. `client/.gitignore`

Remove the now-dead Leptos-specific ignore rules:
- `shells/leptos/generated/`
- `shells/leptos/dist/`
- `shells/leptos/tailwind.css`

(Keep the `shells/swiftui|jetpack-compose|winui/generated/` rules; add
`shelves/react` equivalents only if not already covered by the React shell's own
`.gitignore`, which it is.)

## 5. Root `AGENTS.md`

- §1 table: the `client/` row says "Leptos shell implemented"; update to "React
  shell implemented".
- §2 shell-role parenthetical: "(Leptos now; SwiftUI/WinUI/Compose/Libadwaita
  later)" → "(React now; …)".
- §4 build/run: the `cd client/shells/leptos && trunk serve` example → the React
  dev loop (`cd client/shells/react && pnpm dev`), and the `nix run .#client`
  description (currently "trunk serve the Leptos shell").

## 6. `client/shared/boltffi.toml`

A stale comment above `[targets.wasm]` still says "the Leptos shell drives
`CoreFfi` in-process" — update to reflect that the React shell consumes it and
the Leptos shell is gone.

## Validation (DoD)

- `cargo build` + `cargo test` still pass in the client workspace (or `cargo
  build --manifest-path client/Cargo.toml` with the Leptos member removed).
- `cargo build --manifest-path client/shared/Cargo.toml` still builds (`boltffi
  pack wasm` still produces `shells/react/generated/pkg`).
- `nix flake check` / `nix flake show` still evaluate after the `flake.nix`
  edits.

## Not my scope (already done by the shell agent)

`client/shells/leptos/` deletion + `docs/client/{SHELLS,ARCHITECTURE,CLIENT}.md`
updates + `STYLING_MIGRATION.md` deletion are committed under `71f8763`.
