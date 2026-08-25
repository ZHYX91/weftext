import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  publicDir: "../../prototypes/webui/public",
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: {
      "@codemirror/language": fileURLToPath(new URL("./node_modules/@codemirror/language", import.meta.url)),
      "@codemirror/state": fileURLToPath(new URL("./node_modules/@codemirror/state", import.meta.url)),
      "@codemirror/view": fileURLToPath(new URL("./node_modules/@codemirror/view", import.meta.url)),
      codemirror: fileURLToPath(new URL("./node_modules/codemirror", import.meta.url)),
      react: fileURLToPath(new URL("./node_modules/react", import.meta.url)),
      "react-dom": fileURLToPath(new URL("./node_modules/react-dom", import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    host: host || false,
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
});
