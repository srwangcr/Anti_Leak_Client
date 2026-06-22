import { defineConfig } from 'vite';

export default defineConfig({
  root: 'src',
  server: {
    port: 5173,
    strictPort: false,
  },
  build: {
    outDir: '../dist',
    target: 'ES2020',
  },
});
