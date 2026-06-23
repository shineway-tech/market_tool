import { defineConfig } from "vite";
import packageInfo from "./package.json" with { type: "json" };

export default defineConfig({
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(packageInfo.version),
  },
  server: {
    strictPort: true,
  },
});
