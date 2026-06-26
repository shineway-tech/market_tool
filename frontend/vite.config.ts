import { defineConfig } from "vite";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { readFileSync } from "node:fs";

const frontendRoot = dirname(fileURLToPath(import.meta.url));
const packageJson = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8"));

export default defineConfig({
  root: frontendRoot,
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(packageJson.version),
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    strictPort: true,
  },
});
