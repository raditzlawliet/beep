import tailwindcss from "@tailwindcss/vite";
import { execSync } from "node:child_process";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

// Version: CI sets VITE_BEEP_VERSION from git tag, local dev uses fallback
const version = process.env.VITE_BEEP_VERSION || "0.0.0-dev";

// Git SHA: CI passes VITE_GIT_SHA, local dev reads from git
let gitSha = process.env.VITE_GIT_SHA;
if (!gitSha) {
  try {
    gitSha = execSync("git rev-parse HEAD", {
      encoding: "utf-8",
    }).trim();
  } catch {
    gitSha = "unknown";
  }
}

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [tailwindcss(), sveltekit()],
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,

  define: {
    "import.meta.env.VITE_BEEP_VERSION": JSON.stringify(version),
    "import.meta.env.VITE_GIT_SHA": JSON.stringify(gitSha),
  },

  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
