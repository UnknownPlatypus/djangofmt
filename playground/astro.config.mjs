import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  vite: {
    plugins: [tailwindcss()],
    build: {
      chunkSizeWarningLimit: 4000,
    },
  },
  base: "/djangofmt/",
});
