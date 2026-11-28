import { defineConfig } from 'vite';
import Icons from 'unplugin-icons/vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'node:path';

/**
 * Emits a relative-path (`base: './'`) static web bundle from `apps/desktop/ui`
 * for embedding inside `apps/mobile` WebView shells and single-port reverse proxies.
 */
export default defineConfig({
  base: './',
  plugins: [react(), tailwindcss(), Icons({ compiler: 'jsx', jsx: 'react' })],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  build: {
    outDir: 'dist-mobile-web',
    emptyOutDir: true,
  },
});

