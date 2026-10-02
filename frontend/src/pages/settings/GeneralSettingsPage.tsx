import DeleteProjectSection from '@/components/DeleteProjectSection';
import ProjectGeneralSettings from '@/components/ProjectGeneralSettings';
import { useDashboard } from '@/context/DashboardContext';
import { SettingsLoadState } from './ProjectSettingsLayout';
import { useSettingsContext } from './settingsContext';

function PermPill({ label, on }: { label: string; on: boolean }) {
  return (
    <span
      className={`inline-flex items-center px-2 py-1 rounded-md text-[10px] font-bold uppercase tracking-wide border ${
        on
          ? 'bg-emerald-500/15 text-emerald-900 dark:text-emerald-200 border-emerald-600/35 dark:border-emerald-500/30'
          : 'bg-stitch-elevated/80 dark:bg-white/5 text-stitch-muted border-stitch-border'
      }`}
    >
      {label}
    </span>
  );
}

/** Project settings › General: project properties, the viewer's own permissions and project deletion. */
export default function GeneralSettingsPage() {
  const { projectId: pid, basePath, settings } = useSettingsContext();
  const { csrfToken, refresh } = useDashboard();
  const { perms, members, userLabel } = settings;
  if (settings.loading || settings.error) return <SettingsLoadState settings={settings} />;

  return (
    <div>
      {perms ? (
        <ProjectGeneralSettings
          projectId={pid}
          members={members}
          userLabel={userLabel}
          canEdit={Boolean(perms.manage_project_configuration) && (csrfToken ?? '').length > 0}
          csrfToken={csrfToken ?? ''}
          onSaved={refresh}
        />
      ) : null}

      <section className="mb-10">
        <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-4">
          Your permissions
        </h3>
        {perms ? (
          <div className="flex flex-wrap gap-2">
            <PermPill label="View requirements" on={perms.view_requirements} />
            <PermPill label="Edit requirements" on={perms.edit_requirements} />
            <PermPill label="Approve versions (role)" on={perms.approve_versions} />
            <PermPill
              label="Project reviewer (status / approval)"
              on={perms.is_project_reviewer}
            />
            <PermPill label="Manage custom fields" on={perms.manage_custom_fields} />
            <PermPill label="Manage members" on={perms.manage_project_members} />
          </div>
        ) : null}
      </section>

      <DeleteProjectSection projectId={pid} basePath={basePath} userLabel={userLabel} />
    </div>
  );
}
