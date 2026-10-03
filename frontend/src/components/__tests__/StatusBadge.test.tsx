import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { StatusBadge, statusChipAppearance, statusTagColorSwatchStyle } from '../StatusBadge';

describe('statusTagColorSwatchStyle', () => {
  it('returns undefined for missing or invalid colors', () => {
    expect(statusTagColorSwatchStyle(null)).toBeUndefined();
    expect(statusTagColorSwatchStyle('')).toBeUndefined();
    expect(statusTagColorSwatchStyle('#abc')).toBeUndefined();
  });

  it('returns a backgroundColor for valid hex6', () => {
    expect(statusTagColorSwatchStyle('#AABBCC')).toEqual({ backgroundColor: '#AABBCC' });
  });
});

describe('StatusBadge', () => {
  it('uses catalog hex color with dark text on light backgrounds', () => {
    render(<StatusBadge title="Custom" tagColor="#EEEEEE" />);
    const el = screen.getByText('Custom');
    expect(el).toHaveStyle({ backgroundColor: '#EEEEEE' });
  });

  it('uses catalog hex color with light text on dark backgrounds', () => {
    render(<StatusBadge title="Dark" tagColor="#112233" />);
    const el = screen.getByText('Dark');
    expect(el).toHaveStyle({ backgroundColor: '#112233' });
  });

  it('applies keyword classes for known titles without tag color', () => {
    const { rerender } = render(<StatusBadge title="Approved" />);
    expect(screen.getByText('Approved').className).toMatch(/bg-stitch-accent-dim/);

    rerender(<StatusBadge title="Failed" />);
    expect(screen.getByText('Failed').className).toMatch(/bg-red-500/);

    rerender(<StatusBadge title="Pending review" />);
    expect(screen.getByText('Pending review').className).toMatch(/bg-amber-500/);

    rerender(<StatusBadge title="Draft" />);
    expect(screen.getByText('Draft').className).toMatch(/text-stitch-muted/);

    rerender(<StatusBadge title="Verified" />);
    expect(screen.getByText('Verified').className).toMatch(/bg-amber-500/);
  });

  // Issue #363: keyword chips were dark-theme shades only.
  it('gives keyword chips a light-theme and a dark-theme text shade', () => {
    const { rerender } = render(<StatusBadge title="Accepted" />);
    expect(screen.getByText('Accepted')).toHaveClass('text-amber-800', 'dark:text-amber-200');
    rerender(<StatusBadge title="Pending" />);
    expect(screen.getByText('Pending')).toHaveClass('text-amber-800', 'dark:text-amber-200');
    rerender(<StatusBadge title="Rejected" />);
    expect(screen.getByText('Rejected')).toHaveClass('text-red-700', 'dark:text-red-300');
    rerender(<StatusBadge title="Failed" />);
    expect(screen.getByText('Failed')).toHaveClass('text-red-700', 'dark:text-red-300');
    rerender(<StatusBadge title="Draft" />);
    expect(screen.getByText('Draft')).toHaveClass('bg-black/5', 'dark:bg-white/8');
  });

  // Issue #363: a fixed luminance cut-off put white text on mid-tone catalog colours.
  it('picks the text colour with the higher contrast on catalog colours', () => {
    const dark = '#0f172a';
    const light = '#f8fafc';
    const cases: [string, string][] = [
      ['#f97316', dark], // orange
      ['#f59e0b', dark], // amber
      ['#eab308', dark], // yellow
      ['#22c55e', dark], // green
      ['#112233', light], // navy
      ['#7f1d1d', light], // dark red
    ];
    for (const [color, text] of cases) {
      const { unmount } = render(<StatusBadge title={color} tagColor={color} />);
      expect(screen.getByText(color)).toHaveStyle({ color: text });
      unmount();
    }
  });

  it('falls back to a hashed hue style for unknown titles', () => {
    render(<StatusBadge title="CustomStatusXYZ" />);
    const el = screen.getByText('CustomStatusXYZ');
    expect(el.className).toMatch(/border/);
    // Keyword path would add semantic utility classes; hash fallback does not.
    expect(el.className).not.toMatch(/bg-red-500|bg-amber-500|bg-stitch-accent/);
    expect(el.getAttribute('style')).toBeTruthy();
  });
});

describe('statusChipAppearance (shared with the matrix filter chips, issue #361)', () => {
  it('matches what StatusBadge renders', () => {
    for (const [title, tagColor] of [
      ['Accepted', null],
      ['Failed', null],
      ['Draft', null],
      ['Custom', '#f97316'],
      ['Navy', '#112233'],
      ['SomethingElse', null],
    ] as const) {
      const { className, style } = statusChipAppearance(title, tagColor);
      const { unmount } = render(<StatusBadge title={title} tagColor={tagColor} />);
      const el = screen.getByText(title);
      for (const cls of className.split(' ').filter(Boolean)) expect(el).toHaveClass(cls);
      if (style) expect(el).toHaveStyle(style as Record<string, string>);
      unmount();
    }
  });
});
