# Carve-out (shell agent → core agent): restore `binaryen` to `flake.nix` (core scope)

**From:** shell agent
**To:** core agent
**Status:** pending
**Traceability:** Regression introduced by `878081a` ("finish Leptos-shell removal");
               fix re-applied provisionally as `f5ea4d1` (`fix(core): restore binaryen for
               boltffi pack wasm`) pending this carve-out's confirmation.

## Context

`client/shells/react` is now the sole web shell. Its WASM core bindings are
produced by `boltffi pack wasm` (`pnpm run wasm:build` → `cd ../../shared &&
boltffi pack wasm`), which is the **shared FFI-binding step for every web shell**,
not a Leptos-only concern.

`boltffi.toml` sets `profile = "release"` on `[targets.wasm]`, which makes
`boltffi pack wasm` run `wasm-opt` to optimize the emitted WASM. `wasm-opt` is
provided by the `binaryen` package in nixpkgs.

## The bug

`878081a` removed `binaryen` from `flake.nix`'s `client-packages`, labelling it
"Leptos-only". That is incorrect — `binaryen` was **explicitly listed as keep in
the accepted `CARVEOUT_REMOVE_LEPTOS.md` §3** (line 47):

> `binaryen`, `pnpm`, `nodejs`, `typescript`, `boltffi-cli` — keep (React shell).

The result is that `pnpm run wasm:build` now fails:

```
🌐 Packing WASM
   ✓ Building WASM target
   ✓ removed 175 KiB
error: command failed: wasm-opt not found in PATH
```

Because `client/shells/react/generated/` is gitignored and regenerated on build,
any fresh checkout / rebuild leaves it empty, so the React shell cannot import
`shared` / `shared_types` and the front-end stops working.

## What to do

1. Confirm `binaryen` is present in `flake.nix` `client-packages` (it was
   re-added in `f5ea4d1` alongside `wasm-bindgen-cli`, with a comment clarifying
   both are `boltffi pack wasm` requirements shared by *every* web shell).
2. Re-verify the full WASM pipeline is green:
   - `nix develop .#client --command sh -c 'wasm-opt --version'`
   - `cd client/shells/react && nix develop ../../.#client --command pnpm run wasm:build`
   - `cd client/shells/react && nix develop ../../.#client --command pnpm run build`
   - `nix flake check`

## Not my scope

Nothing else. The `gloo-net` / `trunk` / `wasm-pack` / `tailwindcss_4` /
`console_error_panic_hook` / `lepticons` / `tw_merge` / `paste` removals in
`878081a` are correct — those genuinely were Leptos-only and nothing in the React
shell's `pnpm run build` pipeline references them.

## Validation (DoD)

- `boltffi pack wasm` succeeds (no `wasm-opt not found`).
- `react-router build` succeeds (WASM + types + client + server bundles).
- `nix flake check` passes.
