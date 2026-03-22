import type { CSSProperties } from 'react';

/** Dark-theme status chips; uses catalog `tag_color` (#RRGGBB) when set. */
const HEX6 = /^#[0-9A-Fa-f]{6}$/;

function parseHex6(hex: string): { r: number; g: number; b: number } | null {
  const h = hex.trim();
  if (!HEX6.test(h)) return null;
  return {
    r: parseInt(h.slice(1, 3), 16),
    g: parseInt(h.slice(3, 5), 16),
    b: parseInt(h.slice(5, 7), 16),
  };
}

/** sRGB relative luminance 0..1 */
function relLuminance(r: number, g: number, b: number): number {
  const lin = (c: number) => {
    const x = c / 255;
    return x <= 0.03928 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
  };
  const R = lin(r);
  const G = lin(g);
  const B = lin(b);
  return 0.2126 * R + 0.7152 * G + 0.0722 * B;
}

/**
 * Keyword-based styling when catalog `tag_color` is unset. Only titles containing
 * these English substrings get semantic colors; others used to all look identical (gray).
 */
function classesForTitle(title: string): string | null {
  const t = title.toLowerCase();
  if (t.includes('approved')) {
    return 'bg-stitch-accent-dim text-stitch-on-accent border-transparent';
  }
  if (t.includes('verified') || t.includes('accepted')) {
    return 'bg-amber-500/20 text-amber-200 border-amber-500/30';
  }
  if (t.includes('fail') || t.includes('reject')) {
    return 'bg-red-500/15 text-red-300 border-red-500/25';
  }
  if (t.includes('review') || t.includes('pending')) {
    return 'bg-amber-500/15 text-amber-200 border-amber-500/25';
  }
  if (t.includes('draft')) {
    return 'bg-white/8 text-stitch-muted border-stitch-border';
  }
  return null;
}

/** Stable hue 0–359 from title so each uncatalogued status gets its own muted chip. */
function hashHue(title: string): number {
  let h = 0;
  for (let i = 0; i < title.length; i++) {
    h = (h * 31 + title.charCodeAt(i)) | 0;
  }
  return Math.abs(h) % 360;
}

function hashFallbackStyle(title: string): CSSProperties {
  const hue = hashHue(title);
  return {
    backgroundColor: `hsl(${hue} 32% 24%)`,
    color: 'rgba(248, 250, 252, 0.92)',
    borderColor: `hsl(${hue} 38% 38%)`,
  };
}

/** Inline style for a small swatch next to status selects (create/edit forms). */
export function statusTagColorSwatchStyle(
  tagColor: string | null | undefined,
): CSSProperties | undefined {
  const raw = (tagColor ?? '').trim();
  if (!HEX6.test(raw)) return undefined;
  return { backgroundColor: raw };
}

export function StatusBadge({
  title,
  tagColor,
}: {
  title: string;
  /** Requirement / verification status catalog color (#RRGGBB). */
  tagColor?: string | null;
}) {
  const raw = (tagColor ?? '').trim();
  const rgb = parseHex6(raw);
  if (rgb) {
    const L = relLuminance(rgb.r, rgb.g, rgb.b);
    const fg = L > 0.55 ? '#0f172a' : '#f8fafc';
    const border = L > 0.55 ? 'rgba(15,23,42,0.2)' : 'rgba(248,250,252,0.25)';
    return (
      <span
        className="px-2 py-0.5 rounded-md text-[11px] font-semibold border"
        style={{
          backgroundColor: raw,
          color: fg,
          borderColor: border,
        }}
      >
        {title}
      </span>
    );
  }
  const cls = classesForTitle(title);
  if (cls) {
    return (
      <span className={`px-2 py-0.5 rounded-md text-[11px] font-semibold border ${cls}`}>{title}</span>
    );
  }
  return (
    <span
      className="px-2 py-0.5 rounded-md text-[11px] font-semibold border"
      style={hashFallbackStyle(title)}
    >
      {title}
    </span>
  );
}
