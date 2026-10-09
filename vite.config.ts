import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed dev port. The Rust re-implementation of these commands
// (used in the browser dev preview) is loaded from `src/dev/browse-rust.mjs`.
export default defineConfig({
  plugins: [
    react(),
    {
      name: "cc2027-browser-dev",
      transform(code, id) {
        if (id.endsWith("/src/tauri/bridge.ts")) {
          return code.replace(
            'from "./load-bridge"',
            'from "./browse-rust.mjs"',
          );
        }
        return null;
      },
    },
  ],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "es2021",
  },
});
