import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { StatusBadge, statusTagColorSwatchStyle } from '../StatusBadge';

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

  it('falls back to a hashed hue style for unknown titles', () => {
    render(<StatusBadge title="CustomStatusXYZ" />);
    const el = screen.getByText('CustomStatusXYZ');
    expect(el.className).toMatch(/border/);
    // Keyword path would add semantic utility classes; hash fallback does not.
    expect(el.className).not.toMatch(/bg-red-500|bg-amber-500|bg-stitch-accent/);
    expect(el.getAttribute('style')).toBeTruthy();
  });
});
