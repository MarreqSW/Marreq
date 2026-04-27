import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: __dirname,
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './frontend/static/js'),
      '@modules': path.resolve(__dirname, './frontend/static/js/modules'),
      '@pages': path.resolve(__dirname, './frontend/static/js/pages'),
      '@core': path.resolve(__dirname, './frontend/static/js/core'),
      '@presenters': path.resolve(__dirname, './frontend/static/js/presenters'),
    },
  },
  publicDir: false, // Disable special handling of 'public' directory
  server: {
    fs: {
      strict: false,
      allow: [__dirname, path.resolve(__dirname, 'frontend/static')],
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./marreq-core/tests/js/setup.js'],
    exclude: ['**/node_modules/**', '**/dist/**'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      include: ['frontend/static/js/**/*.js'],
      exclude: [
        'frontend/static/js/**/*.test.js',
        'frontend/static/js/**/*.spec.js',
      ],
    },
  },
});
