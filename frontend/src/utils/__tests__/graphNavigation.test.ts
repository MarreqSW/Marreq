import { describe, it, expect } from 'vitest';
import { entityDetailPath, parseGraphNodeId } from '../graphNavigation';

describe('parseGraphNodeId', () => {
  it('parses requirement node ids', () => {
    expect(parseGraphNodeId('r-42')).toEqual({
      kind: 'requirement',
      entityId: 42,
    });
  });

  it('parses verification node ids', () => {
    expect(parseGraphNodeId('v-7')).toEqual({
      kind: 'verification',
      entityId: 7,
    });
  });

  it('returns null for invalid ids', () => {
    expect(parseGraphNodeId('x-1')).toBeNull();
    expect(parseGraphNodeId('r-')).toBeNull();
    expect(parseGraphNodeId('r-abc')).toBeNull();
    expect(parseGraphNodeId('')).toBeNull();
  });
});

describe('entityDetailPath', () => {
  const base = '/acme/my-project';

  it('builds requirement view path', () => {
    expect(entityDetailPath(base, 'requirement', 42)).toBe(
      '/acme/my-project/requirements/42',
    );
  });

  it('builds verification view path', () => {
    expect(entityDetailPath(base, 'verification', 7)).toBe(
      '/acme/my-project/verifications/7',
    );
  });

  it('strips trailing slash from basePath', () => {
    expect(entityDetailPath(`${base}/`, 'requirement', 1)).toBe(
      '/acme/my-project/requirements/1',
    );
  });
});
