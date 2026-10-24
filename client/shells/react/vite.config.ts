import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [wasm(), tailwindcss(), reactRouter()],
  resolve: {
    tsconfigPaths: true,
  },
});
