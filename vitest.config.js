import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: __dirname,
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src/html/static/js'),
      '@modules': path.resolve(__dirname, './src/html/static/js/modules'),
      '@pages': path.resolve(__dirname, './src/html/static/js/pages'),
      '@core': path.resolve(__dirname, './src/html/static/js/core'),
      '@presenters': path.resolve(__dirname, './src/html/static/js/presenters'),
    },
  },
  publicDir: false, // Disable special handling of 'public' directory
  server: {
    fs: {
      strict: false,
      allow: [__dirname, path.resolve(__dirname, 'src')],
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./tests/js/setup.js'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      include: ['src/html/static/js/**/*.js'],
      exclude: [
        'src/html/static/js/**/*.test.js',
        'src/html/static/js/**/*.spec.js',
      ],
    },
  },
});
