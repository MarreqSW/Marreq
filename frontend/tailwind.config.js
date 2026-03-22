/** @type {import('tailwindcss').Config} */
/** Semantic stitch palette via CSS variables (see index.css :root / html.dark) */
export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        surface: '#f8f9fb',
        'surface-low': '#f2f4f6',
        'surface-container': '#eceef0',
        'surface-high': '#e6e8ea',
        'surface-highest': '#e0e3e5',
        'on-surface': '#191c1e',
        'on-surface-variant': '#454652',
        primary: '#000666',
        'primary-container': '#1a237e',
        outline: '#767683',
        'outline-variant': '#c6c5d4',
        tertiary: '#380b00',
        'tertiary-container': '#5c1800',
        'on-tertiary-fixed-variant': '#7b2e12',
        secondary: '#585c80',
        error: '#ba1a1a',
        'error-container': '#ffdad6',
        'on-error-container': '#93000a',
        'on-tertiary-container': '#e17c5a',
        /** Theme-aware UI (variables switch with html.dark) */
        stitch: {
          canvas: 'var(--stitch-canvas)',
          surface: 'var(--stitch-surface)',
          elevated: 'var(--stitch-elevated)',
          higher: 'var(--stitch-higher)',
          muted: 'var(--stitch-muted)',
          subtle: 'var(--stitch-subtle)',
          border: 'var(--stitch-border)',
          accent: 'var(--stitch-accent)',
          'accent-dim': 'var(--stitch-accent-dim)',
          danger: 'var(--stitch-danger)',
          fg: 'var(--stitch-fg)',
          'fg-secondary': 'var(--stitch-fg-secondary)',
          'on-accent': 'var(--stitch-on-accent)',
        },
      },
      fontFamily: {
        sans: ['"Google Sans Text"', 'Inter', 'system-ui', 'sans-serif'],
        headline: ['"Public Sans"', 'Inter', 'system-ui', 'sans-serif'],
        body: ['"Google Sans Text"', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'monospace'],
      },
      fontSize: {
        stitch: ['13px', { lineHeight: '1.35' }],
      },
      borderRadius: {
        md: '0.375rem',
        lg: '0.25rem',
      },
      boxShadow: {
        stitch: '0px 4px 20px rgba(0, 0, 0, 0.35), 0px 2px 8px rgba(0, 0, 0, 0.25)',
        'stitch-inset': 'inset 0 -2px 0 rgba(255,255,255,0.06)',
      },
    },
  },
  plugins: [],
};
