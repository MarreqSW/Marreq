import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: __dirname,
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './backend/src/html/static/js'),
      '@modules': path.resolve(__dirname, './backend/src/html/static/js/modules'),
      '@pages': path.resolve(__dirname, './backend/src/html/static/js/pages'),
      '@core': path.resolve(__dirname, './backend/src/html/static/js/core'),
      '@presenters': path.resolve(__dirname, './backend/src/html/static/js/presenters'),
    },
  },
  publicDir: false, // Disable special handling of 'public' directory
  server: {
    fs: {
      strict: false,
      allow: [__dirname, path.resolve(__dirname, 'backend/src')],
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./backend/tests/js/setup.js'],
    exclude: ['**/node_modules/**', '**/dist/**'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      include: ['backend/src/html/static/js/**/*.js'],
      exclude: [
        'backend/src/html/static/js/**/*.test.js',
        'backend/src/html/static/js/**/*.spec.js',
      ],
    },
  },
});
