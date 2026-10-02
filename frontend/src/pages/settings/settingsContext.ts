import { useOutletContext } from 'react-router-dom';
import type { EffectivePermissions, ProjectMember, User } from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';

/** Data the settings tabs share, loaded once by `ProjectSettingsLayout`. */
export type ProjectSettingsData = {
  loading: boolean;
  error: string | null;
  perms: EffectivePermissions | null;
  members: ProjectMember[];
  /** All accounts; `null` when the user directory is not visible (non-admins). */
  users: User[] | null;
  userLabel: (userId: number) => string;
  /** Reload members and permissions (e.g. after a role change). */
  reload: () => Promise<void>;
};

export type SettingsOutletContext = ProjectOutletContext & { settings: ProjectSettingsData };

export function useSettingsContext(): SettingsOutletContext {
  return useOutletContext<SettingsOutletContext>();
}
