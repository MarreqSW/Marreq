import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
import { getCsrfToken, getDashboard, logoutJson } from '@/api/client';
import type { DashboardPayload, DashboardProject } from '@/api/types';

type DashboardContextValue = {
  dashboard: DashboardPayload | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  setSelectedProjectId: (id: number) => void;
  logout: () => Promise<void>;
  csrfToken: string | null;
};

const DashboardContext = createContext<DashboardContextValue | null>(null);

export function DashboardProvider({ children }: { children: ReactNode }) {
  const [dashboard, setDashboard] = useState<DashboardPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [csrfToken, setCsrfToken] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const d = await getDashboard();
      setDashboard(d);
      setCsrfToken(d.csrf_token);
    } catch (e) {
      setDashboard(null);
      setCsrfToken(null);
      setError(e instanceof Error ? e.message : 'Failed to load dashboard');
      throw e;
    } finally {
      setLoading(false);
    }
  }, []);

  const setSelectedProjectId = useCallback((id: number) => {
    setDashboard((prev) =>
      prev ? { ...prev, selected_project_id: id } : prev,
    );
    document.cookie = `selected_project_id=${id}; path=/; SameSite=Lax`;
  }, []);

  const logout = useCallback(async () => {
    const token = csrfToken ?? (await getCsrfToken());
    await logoutJson(token);
    setDashboard(null);
    setCsrfToken(null);
  }, [csrfToken]);

  const value = useMemo(
    () => ({
      dashboard,
      loading,
      error,
      refresh,
      setSelectedProjectId,
      logout,
      csrfToken,
    }),
    [dashboard, loading, error, refresh, setSelectedProjectId, logout, csrfToken],
  );

  return (
    <DashboardContext.Provider value={value}>{children}</DashboardContext.Provider>
  );
}

export function useDashboard() {
  const ctx = useContext(DashboardContext);
  if (!ctx) throw new Error('useDashboard outside DashboardProvider');
  return ctx;
}

export function useProjects(): DashboardProject[] {
  return useDashboard().dashboard?.projects ?? [];
}
