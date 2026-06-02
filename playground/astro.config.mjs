import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import wasmPack from "./vite-plugin-wasm.mjs";

export default defineConfig({
  vite: {
    plugins: [tailwindcss(), wasmPack()],
    build: { chunkSizeWarningLimit: 4000 },
    server: { watch: { ignored: ["**/pkg/**"] } },
  },
  base: "/djangofmt/",
});
