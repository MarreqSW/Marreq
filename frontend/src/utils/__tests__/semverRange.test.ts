import { describe, expect, it } from 'vitest';
import { isVersionInInclusiveRange, shortSha } from '../semverRange';

describe('isVersionInInclusiveRange', () => {
  it('accepts versions on the inclusive bounds', () => {
    expect(isVersionInInclusiveRange('0.1.0', '0.1.0', '0.1.99')).toBe(true);
    expect(isVersionInInclusiveRange('0.1.99', '0.1.0', '0.1.99')).toBe(true);
  });

  it('accepts versions inside the range', () => {
    expect(isVersionInInclusiveRange('0.1.5', '0.1.0', '0.1.99')).toBe(true);
  });

  it('rejects versions outside the range', () => {
    expect(isVersionInInclusiveRange('0.0.9', '0.1.0', '0.1.99')).toBe(false);
    expect(isVersionInInclusiveRange('0.2.0', '0.1.0', '0.1.99')).toBe(false);
  });

  it('rejects malformed semver', () => {
    expect(isVersionInInclusiveRange('1.0', '0.1.0', '0.1.99')).toBe(false);
    expect(isVersionInInclusiveRange('0.1.0', 'bad', '0.1.99')).toBe(false);
  });
});

describe('shortSha', () => {
  it('truncates to 7 characters', () => {
    expect(shortSha('56c09392abcdef')).toBe('56c0939');
  });

  it('returns null for unknown or empty SHAs', () => {
    expect(shortSha('unknown')).toBeNull();
    expect(shortSha('')).toBeNull();
    expect(shortSha(undefined)).toBeNull();
  });
});
