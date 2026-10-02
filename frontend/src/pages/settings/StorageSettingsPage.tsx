import ProjectStorageSettings from '@/components/ProjectStorageSettings';
import { useDashboard } from '@/context/DashboardContext';
import { parseUser } from '@/utils/parseUser';
import { useSettingsContext } from './settingsContext';

/** Project settings › Storage: attachment usage and quota. */
export default function StorageSettingsPage() {
  const { projectId: pid } = useSettingsContext();
  const { dashboard, csrfToken } = useDashboard();
  return (
    <ProjectStorageSettings
      projectId={pid}
      isAdmin={Boolean(parseUser(dashboard?.user)?.is_admin)}
      csrfToken={csrfToken ?? ''}
    />
  );
}
