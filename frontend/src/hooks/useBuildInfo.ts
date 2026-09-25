import { useEffect, useState } from 'react';
import type { BuildInfo } from '../api/types';
import { getBuildInfo } from '../api/client';

// One `/api/meta/build` request shared by the sidebar, Help page, login page and banner.
let cached: Promise<BuildInfo> | null = null;

export function loadBuildInfo(): Promise<BuildInfo> {
  if (!cached) {
    const pending = Promise.resolve()
      .then(() => getBuildInfo())
      .then((info) => {
        if (!info) throw new Error('empty build info');
        return info;
      });
    // Drop failures so a later mount can retry.
    pending.catch(() => {
      if (cached === pending) cached = null;
    });
    cached = pending;
  }
  return cached;
}

/** Test helper: forget the cached build info. */
export function resetBuildInfoCache() {
  cached = null;
}

export function useBuildInfo(): { build: BuildInfo | null; error: boolean } {
  const [build, setBuild] = useState<BuildInfo | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let alive = true;
    loadBuildInfo()
      .then((info) => {
        if (alive) setBuild(info);
      })
      .catch(() => {
        if (alive) setError(true);
      });
    return () => {
      alive = false;
    };
  }, []);

  return { build, error };
}
