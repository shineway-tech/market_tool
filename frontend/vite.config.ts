import { defineConfig } from "vite";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const frontendRoot = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: frontendRoot,
  clearScreen: false,
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    strictPort: true,
  },
});
