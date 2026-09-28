import { defineConfig } from 'vite';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    'process.env.GITHUB_OWNER': JSON.stringify(process.env.GITHUB_OWNER || 'civi0411'),
    'process.env.GITHUB_REPO': JSON.stringify(process.env.GITHUB_REPO || 'Custos'),
    'process.env.CUSTOS_BUNDLE_NAME': JSON.stringify(
      process.env.CUSTOS_BUNDLE_NAME || process.env.GOOSE_BUNDLE_NAME || 'Custos',
    ),
    'process.env.GOOSE_BUNDLE_NAME': JSON.stringify(process.env.GOOSE_BUNDLE_NAME),
  },
});
