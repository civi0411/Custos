import { defineConfig } from "vite";
import Icons from "unplugin-icons/vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
// @ts-expect-error type error without @types/node package
import { fileURLToPath, URL } from "node:url";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), tailwindcss(), Icons({ compiler: "jsx", jsx: "react" })],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            { name: 'settings', test: /\/src\/(?:features\/ai-setup|components\/modals)\//, maxSize: 250_000 },
            { name: 'syntax', test: /node_modules\/(?:highlight\.js|prism-react-renderer)\//, maxSize: 300_000 },
            { name: 'math', test: /node_modules\/(?:katex|rehype-katex|remark-math)\//, maxSize: 300_000 },
            { name: 'react', test: /node_modules\/(?:react|react-dom|scheduler)\//, maxSize: 300_000 },
            { name: 'vendor', test: /node_modules\//, priority: -1, maxSize: 300_000 },
          ],
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
