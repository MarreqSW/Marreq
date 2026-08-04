/** Inclusive min/max semver check (major.minor.patch only; no prerelease). */

function parseSemverTuple(version: string): [number, number, number] | null {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(version.trim());
  if (!m) return null;
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

function compareTuples(a: [number, number, number], b: [number, number, number]): number {
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] < b[i] ? -1 : 1;
  }
  return 0;
}

/** True when `version` is within `[min, max]` inclusive (semver major.minor.patch). */
export function isVersionInInclusiveRange(
  version: string,
  min: string,
  max: string,
): boolean {
  const v = parseSemverTuple(version);
  const lo = parseSemverTuple(min);
  const hi = parseSemverTuple(max);
  if (!v || !lo || !hi) return false;
  return compareTuples(v, lo) >= 0 && compareTuples(v, hi) <= 0;
}

export function getFrontendBuildConstants() {
  return {
    version: __FRONTEND_VERSION__,
    requiresBackendMin: __REQUIRES_BACKEND_MIN__,
    requiresBackendMax: __REQUIRES_BACKEND_MAX__,
    gitSha: __FRONTEND_GIT_SHA__,
  };
}
