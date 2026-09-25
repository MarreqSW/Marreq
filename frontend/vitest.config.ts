/// <reference types="vitest" />
import { readFileSync } from 'node:fs';
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const pkg = JSON.parse(readFileSync(path.resolve(__dirname, 'package.json'), 'utf-8')) as {
  version: string;
};
// Release version from the `marreq-frontend-v*` tag (via MARREQ_VERSION), else package.json.
const frontendVersion = (process.env.MARREQ_VERSION || pkg.version).replace(/^v/, '');
const compatibility = JSON.parse(
  readFileSync(path.resolve(__dirname, 'compatibility.json'), 'utf-8'),
) as {
  requires_backend_min: string;
  requires_backend_max: string;
};

export default defineConfig({
  plugins: [react()],
  define: {
    __FRONTEND_VERSION__: JSON.stringify(frontendVersion),
    __REQUIRES_BACKEND_MIN__: JSON.stringify(compatibility.requires_backend_min),
    __REQUIRES_BACKEND_MAX__: JSON.stringify(compatibility.requires_backend_max),
    __FRONTEND_GIT_SHA__: JSON.stringify(process.env.MARREQ_GIT_SHA ?? 'unknown'),
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'text-summary', 'html', 'lcov', 'json-summary', 'cobertura'],
      reportsDirectory: './coverage',
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/**/*.{test,spec}.{ts,tsx}',
        'src/test/**',
        'src/main.tsx',
      ],
    },
  },
});
