import { useEffect, useState } from 'react';
import { getBuildInfo } from '@/api/client';
import type { BuildInfo } from '@/api/types';
import {
  getFrontendBuildConstants,
  isVersionInInclusiveRange,
} from '@/utils/semverRange';

type CompatState =
  | { status: 'ok' | 'loading' | 'unavailable' }
  | { status: 'mismatch'; build: BuildInfo };

/**
 * Non-blocking amber banner when SPA ↔ API versions fall outside declared ranges.
 */
export default function CompatibilityBanner() {
  const [state, setState] = useState<CompatState>({ status: 'loading' });
  const ui = getFrontendBuildConstants();

  useEffect(() => {
    let alive = true;
    getBuildInfo()
      .then((build) => {
        if (!alive) return;
        const uiOk = isVersionInInclusiveRange(
          ui.version,
          build.frontend_compatibility.min_version,
          build.frontend_compatibility.max_version,
        );
        const apiOk = isVersionInInclusiveRange(
          build.backend_version,
          ui.requiresBackendMin,
          ui.requiresBackendMax,
        );
        setState(uiOk && apiOk ? { status: 'ok' } : { status: 'mismatch', build });
      })
      .catch(() => {
        if (alive) setState({ status: 'unavailable' });
      });
    return () => {
      alive = false;
    };
  }, [ui.version, ui.requiresBackendMin, ui.requiresBackendMax]);

  if (state.status !== 'mismatch') {
    return null;
  }

  const { build } = state;
  return (
    <div
      role="status"
      className="border-b border-amber-500/40 bg-amber-500/15 px-4 py-2 text-sm text-amber-950 dark:text-amber-100"
    >
      <p className="font-semibold">Version mismatch</p>
      <p className="mt-0.5 text-xs opacity-90">
        UI {ui.version} (expects API {ui.requiresBackendMin}–{ui.requiresBackendMax}) · API{' '}
        {build.backend_version} (expects UI {build.frontend_compatibility.min_version}–
        {build.frontend_compatibility.max_version}). Upgrade or align deployments.
      </p>
    </div>
  );
}
