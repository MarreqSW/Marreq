import { useCallback, useEffect, useMemo, useState } from 'react';
import { Outlet, useOutletContext } from 'react-router-dom';
import { getMyPermissions, listProjectMembers, listUsersOptional } from '@/api/client';
import type { EffectivePermissions, ProjectMember, User } from '@/api/types';
import StitchPageHeader from '@/components/StitchPageHeader';
import SubNav, { type SubNavItem } from '@/components/SubNav';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { formatUserLabel } from '@/utils/userLabel';
import type { ProjectSettingsData, SettingsOutletContext } from './settingsContext';

export const SETTINGS_SECTIONS: SubNavItem[] = [
  { to: 'general', label: 'General', icon: 'tune' },
  { to: 'members', label: 'Members & reviewers', icon: 'group' },
  { to: 'catalog', label: 'Catalog', icon: 'category' },
  { to: 'storage', label: 'Storage', icon: 'hard_drive' },
  { to: 'notifications', label: 'Notifications', icon: 'notifications' },
  { to: 'import', label: 'Import', icon: 'upload_file' },
];

/** Project settings hub: one header and tabs over the configuration pages (issue #346). */
export default function ProjectSettingsLayout() {
  const projectContext = useOutletContext<ProjectOutletContext>();
  const { projectId: pid } = projectContext;
  const { dashboard } = useDashboard();
  const projectName = dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setError(null);
    try {
      const [p, m, u] = await Promise.all([
        getMyPermissions(pid),
        listProjectMembers(pid),
        listUsersOptional(),
      ]);
      setPerms(p ?? null);
      setMembers(m ?? []);
      setUsers(u ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load settings');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    setLoading(true);
    void reload();
  }, [reload]);

  const settings: ProjectSettingsData = useMemo(
    () => ({
      loading,
      error,
      perms,
      members,
      users,
      userLabel: (uid: number) => formatUserLabel(uid, { users, members }),
      reload,
    }),
    [loading, error, perms, members, users, reload],
  );

  const context: SettingsOutletContext = { ...projectContext, settings };

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Settings"
        title="Project settings"
        subtitle="Properties, members and reviewers, catalog, storage, notifications and imports for this project."
      />
      <SubNav items={SETTINGS_SECTIONS} ariaLabel="Project settings sections" />
      <Outlet context={context} />
    </div>
  );
}

/** Placeholder for tabs that need the shared data. */
export function SettingsLoadState({ settings }: { settings: ProjectSettingsData }) {
  if (settings.error) {
    return (
      <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-700 dark:text-red-200 text-sm">
        {settings.error}
      </div>
    );
  }
  return (
    <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
      Loading settings…
    </div>
  );
}
