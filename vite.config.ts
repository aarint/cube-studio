import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";

// Tauri serves the frontend from a fixed port in development; that exact URL is
// what `tauri.conf.json` declares as `devUrl`.
const DEV_PORT = 1420;
const rootDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(rootDir, "src"),
    },
  },
  // Tauri needs a fixed port, and must fail rather than silently pick another.
  server: {
    port: DEV_PORT,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      // The Rust side has its own rebuild loop.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    // Match the WebView versions Tauri ships on each platform.
    target: ["es2021", "chrome105", "safari14"],
    sourcemap: true,
    outDir: "dist",
    emptyOutDir: true,
  },
});
