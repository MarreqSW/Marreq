import { useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { setVisitDetail } from '@/utils/recentPages';

/** Name what this page shows (e.g. a reference code) in the start screen's Recent list. */
export function useRecentDetail(projectId: number, detail: string | null | undefined) {
  const location = useLocation();
  const path = location.pathname + location.search;
  useEffect(() => {
    if (detail) setVisitDetail(projectId, path, detail);
  }, [projectId, path, detail]);
}
