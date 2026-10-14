import { defineConfig } from "vite";

const apiPort = Number(process.env.WEFTEXT_API_PORT ?? "47821");
if (!Number.isInteger(apiPort) || apiPort < 1 || apiPort > 65535) throw new Error("WEFTEXT_API_PORT must be a local port number");
const apiTarget = `http://127.0.0.1:${apiPort}`;

export default defineConfig({
  server: { proxy: { "/api": { target: apiTarget, changeOrigin: true, configure(proxy) { proxy.on("proxyReq", (request, incoming) => { if (incoming.headers.origin) request.setHeader("Origin", apiTarget); }); } } } },
  build: { target: "es2022", rollupOptions: { input: { main: "index.html", hosted: "hosted.html" } } },
});
