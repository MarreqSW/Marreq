import type { IncomingMessage } from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

type HttpProxyLike = {
  on(event: 'proxyRes', listener: (proxyRes: IncomingMessage) => void): void;
};

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/** Shared by dev server and `vite preview` so `/api/*` always reaches Rocket when testing a production build locally. */
const apiProxy = {
  '/api': {
    target: 'http://127.0.0.1:8000',
    changeOrigin: true,
    // Rocket may emit Set-Cookie with Domain/Secure derived from the proxied host.
    // Strip those so the browser stores session/csrf cookies for the Vite origin (e.g. :5173 / :4173).
    configure: (proxy: HttpProxyLike) => {
      proxy.on('proxyRes', (proxyRes) => {
        const raw = proxyRes.headers['set-cookie'];
        if (raw == null) return;
        const cookies = Array.isArray(raw) ? raw : [raw];
        proxyRes.headers['set-cookie'] = cookies.map((cookie) => {
          let c = cookie
            .replace(/;\s*Domain=[^;]*/gi, '')
            .replace(/;\s*Secure/gi, '');
          if (/;\s*Path=/i.test(c)) {
            c = c.replace(/;\s*Path=[^;]*/gi, '; Path=/');
          } else {
            c += '; Path=/';
          }
          return c;
        });
      });
    },
  },
};

export default defineConfig({
  root: '.',
  publicDir: 'public',
  // Ensure `/projects`, `/user/...`, etc. serve `index.html` in dev so the SPA shell runs.
  appType: 'spa',
  plugins: [react()],
  server: {
    port: 5173,
    proxy: apiProxy,
    fs: {
      allow: [path.resolve(__dirname, '..')],
    },
  },
  preview: {
    proxy: apiProxy,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      '@static': path.resolve(__dirname, 'static'),
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    emptyOutDir: true,
  },
});
