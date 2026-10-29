import { defineConfig } from 'vitest/config';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: __dirname,
  test: {
    name: 'e2e',
    include: ['tests/e2e/**/*.e2e.test.ts'],
    exclude: ['node_modules', 'dist', 'build'],
    environment: 'node',
    setupFiles: ['./tests/e2e/setup.ts'],
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
