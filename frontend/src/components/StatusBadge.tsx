import type { CSSProperties } from 'react';

/** Status chips for both themes; uses catalog `tag_color` (#RRGGBB) when set. */
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

/** WCAG contrast ratio between two relative luminances. */
function contrastRatio(a: number, b: number): number {
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

const DARK_TEXT = { hex: '#0f172a', luminance: relLuminance(0x0f, 0x17, 0x2a) };
const LIGHT_TEXT = { hex: '#f8fafc', luminance: relLuminance(0xf8, 0xfa, 0xfc) };

/**
 * Keyword-based styling when catalog `tag_color` is unset. Only titles containing
 * these English substrings get semantic colors; others used to all look identical (gray).
 * Each text colour has a light-theme and a dark-theme shade (issue #363).
 */
function classesForTitle(title: string): string | null {
  const t = title.toLowerCase();
  if (t.includes('approved')) {
    return 'bg-stitch-accent-dim text-stitch-on-accent border-transparent';
  }
  if (t.includes('verified') || t.includes('accepted')) {
    return 'bg-amber-500/20 text-amber-800 dark:text-amber-200 border-amber-500/30';
  }
  if (t.includes('fail') || t.includes('reject')) {
    return 'bg-red-500/15 text-red-700 dark:text-red-300 border-red-500/25';
  }
  if (t.includes('review') || t.includes('pending')) {
    return 'bg-amber-500/15 text-amber-800 dark:text-amber-200 border-amber-500/25';
  }
  if (t.includes('draft')) {
    return 'bg-black/5 dark:bg-white/8 text-stitch-muted border-stitch-border';
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

/**
 * Colours of a status chip: the catalog colour (with the better-contrast
 * text), else keyword classes, else a hashed hue. Shared by `StatusBadge` and
 * the matrix status filter chips so both always look the same.
 */
export function statusChipAppearance(
  title: string,
  tagColor?: string | null,
): { className: string; style?: CSSProperties } {
  const raw = (tagColor ?? '').trim();
  const rgb = parseHex6(raw);
  if (rgb) {
    // The text colour with the higher contrast on the catalog colour (a fixed
    // luminance cut-off put white text on mid-tones such as orange or amber).
    const L = relLuminance(rgb.r, rgb.g, rgb.b);
    const darkText =
      contrastRatio(L, DARK_TEXT.luminance) >= contrastRatio(L, LIGHT_TEXT.luminance);
    return {
      className: '',
      style: {
        backgroundColor: raw,
        color: darkText ? DARK_TEXT.hex : LIGHT_TEXT.hex,
        borderColor: darkText ? 'rgba(15,23,42,0.2)' : 'rgba(248,250,252,0.25)',
      },
    };
  }
  const cls = classesForTitle(title);
  if (cls) return { className: cls };
  return { className: '', style: hashFallbackStyle(title) };
}

export function StatusBadge({
  title,
  tagColor,
}: {
  title: string;
  /** Requirement / verification status catalog color (#RRGGBB). */
  tagColor?: string | null;
}) {
  const { className, style } = statusChipAppearance(title, tagColor);
  return (
    <span
      className={`px-2 py-0.5 rounded-md text-[11px] font-semibold border ${className}`.trim()}
      style={style}
    >
      {title}
    </span>
  );
}
