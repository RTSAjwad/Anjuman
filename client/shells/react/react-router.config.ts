import type { Config } from "@react-router/dev/config";

export default {
  // Crux runs client-side (WASM + bincode over the FFI Bridge); the shell is a
  // Single Page App — no server rendering. Pre-renders the root route into an
  // index.html at build time, then hydrates client-side.
  ssr: false,
} satisfies Config;
