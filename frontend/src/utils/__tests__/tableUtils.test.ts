import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { downloadCsv, escapeCsv, paginationItems } from '../tableUtils';

describe('escapeCsv', () => {
  it('returns plain strings unchanged', () => {
    expect(escapeCsv('hello')).toBe('hello');
    expect(escapeCsv('simple text')).toBe('simple text');
  });

  it('quotes strings containing commas', () => {
    expect(escapeCsv('a,b')).toBe('"a,b"');
  });

  it('quotes strings containing newlines', () => {
    expect(escapeCsv('line1\nline2')).toBe('"line1\nline2"');
  });

  it('quotes and doubles embedded quotes', () => {
    expect(escapeCsv('say "hi"')).toBe('"say ""hi"""');
  });
});

describe('paginationItems', () => {
  it('lists all pages when total is small', () => {
    expect(paginationItems(1, 5)).toEqual([1, 2, 3, 4, 5]);
    expect(paginationItems(3, 7)).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });

  it('shows leading window with trailing dots', () => {
    expect(paginationItems(1, 20)).toEqual([1, 2, 3, 4, 5, 'dots', 20]);
    expect(paginationItems(4, 20)).toEqual([1, 2, 3, 4, 5, 'dots', 20]);
  });

  it('shows trailing window with leading dots', () => {
    expect(paginationItems(18, 20)).toEqual([1, 'dots', 16, 17, 18, 19, 20]);
    expect(paginationItems(20, 20)).toEqual([1, 'dots', 16, 17, 18, 19, 20]);
  });

  it('shows middle window with dots on both sides', () => {
    expect(paginationItems(10, 20)).toEqual([1, 'dots', 9, 10, 11, 'dots', 20]);
  });
});

describe('downloadCsv', () => {
  const originalCreateObjectURL = URL.createObjectURL;
  const originalRevokeObjectURL = URL.revokeObjectURL;

  beforeEach(() => {
    URL.createObjectURL = vi.fn(() => 'blob:mock-url');
    URL.revokeObjectURL = vi.fn();
  });

  afterEach(() => {
    URL.createObjectURL = originalCreateObjectURL;
    URL.revokeObjectURL = originalRevokeObjectURL;
    vi.restoreAllMocks();
  });

  it('builds a CSV blob and triggers a download click', () => {
    const click = vi.fn();
    const createElement = vi.spyOn(document, 'createElement').mockImplementation((tag) => {
      if (tag === 'a') {
        return {
          href: '',
          download: '',
          click,
        } as unknown as HTMLAnchorElement;
      }
      return document.createElement(tag);
    });

    downloadCsv('export.csv', ['A', 'B'], [
      ['1', '2'],
      ['3', '4'],
    ]);

    expect(URL.createObjectURL).toHaveBeenCalled();
    expect(click).toHaveBeenCalled();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:mock-url');
    createElement.mockRestore();
  });
});
