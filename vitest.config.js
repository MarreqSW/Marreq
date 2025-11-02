import { defineConfig } from 'vitest/config';

export default defineConfig({
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
