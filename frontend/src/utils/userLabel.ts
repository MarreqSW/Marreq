/** Display name for an author/reviewer id without requiring the admin-only user directory. */

export type UserIdentity = {
  id: number;
  name: string;
  username: string;
};

export type ProjectMemberIdentity = {
  user_id: number;
  name: string;
  username: string;
};

export type UserLabelSources = {
  /** Project members: available to any viewer with ViewRequirements. */
  members?: ReadonlyArray<ProjectMemberIdentity> | null;
  /** Full directory: admin-only; overlays members when present. */
  users?: ReadonlyArray<UserIdentity> | null;
  /** Signed-in session user. */
  me?: UserIdentity | null;
};

export function formatUserLabel(id: number, sources: UserLabelSources = {}): string {
  const byId = new Map<number, UserIdentity>();
  if (sources.members) {
    for (const member of sources.members) {
      byId.set(member.user_id, { id: member.user_id, name: member.name, username: member.username });
    }
  }
  if (sources.users) {
    for (const user of sources.users) {
      byId.set(user.id, { id: user.id, name: user.name, username: user.username });
    }
  }
  if (sources.me) {
    byId.set(sources.me.id, sources.me);
  }
  const match = byId.get(id);
  if (match) {
    return `${match.name} (${match.username})`;
  }
  return `User #${id}`;
}
