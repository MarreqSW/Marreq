import { type FormEvent, useEffect, useMemo, useState } from 'react';
import {
  getProjectReviewers,
  putProjectReviewers,
  removeProjectMember,
  setProjectMemberRole,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import { SettingsLoadState } from './ProjectSettingsLayout';
import { useSettingsContext } from './settingsContext';

const ROLES = [
  { id: 1, label: 'Admin' },
  { id: 2, label: 'Reviewer' },
  { id: 3, label: 'Author' },
  { id: 4, label: 'Viewer' },
];

/** Project settings › Members & reviewers. */
export default function MembersSettingsPage() {
  const { projectId: pid, settings } = useSettingsContext();
  const { csrfToken } = useDashboard();
  const { perms, members, users, userLabel, reload } = settings;
  const [memberErr, setMemberErr] = useState<string | null>(null);
  const [addUserId, setAddUserId] = useState('');
  const [addRole, setAddRole] = useState(4);
  const [memberBusy, setMemberBusy] = useState<number | 'add' | null>(null);
  const [reviewerIds, setReviewerIds] = useState<number[]>([]);
  const [reviewerDraft, setReviewerDraft] = useState<Set<number>>(() => new Set());
  const [reviewerErr, setReviewerErr] = useState<string | null>(null);
  const [reviewerBusy, setReviewerBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    Promise.resolve(getProjectReviewers(pid))
      .catch(() => ({ user_ids: [] as number[] }))
      .then((rev) => {
        if (cancelled) return;
        const ids = rev?.user_ids ?? [];
        setReviewerIds(ids);
        setReviewerDraft(new Set(ids));
      });
    return () => {
      cancelled = true;
    };
  }, [pid]);

  const memberIds = useMemo(() => new Set(members.map((m) => m.user_id)), [members]);

  const usersNotInProject = useMemo(() => {
    if (!users?.length) return [];
    return users.filter((u) => !memberIds.has(u.id));
  }, [users, memberIds]);

  const canManage = perms?.manage_project_members && (csrfToken ?? '').length > 0;
  const reviewersDirty = useMemo(() => {
    if (reviewerIds.length !== reviewerDraft.size) return true;
    const a = [...reviewerIds].sort((x, y) => x - y);
    const b = [...reviewerDraft].sort((x, y) => x - y);
    return a.some((id, i) => id !== b[i]);
  }, [reviewerIds, reviewerDraft]);

  async function saveReviewers() {
    const token = csrfToken ?? '';
    if (!token || !canManage) return;
    setReviewerErr(null);
    setReviewerBusy(true);
    try {
      const next = [...reviewerDraft].sort((a, b) => a - b);
      const res = await putProjectReviewers(pid, next, token);
      setReviewerIds(res.user_ids);
      setReviewerDraft(new Set(res.user_ids));
    } catch (e) {
      setReviewerErr(e instanceof Error ? e.message : 'Failed to save reviewers');
    } finally {
      setReviewerBusy(false);
    }
  }

  function toggleReviewerDraft(userId: number, on: boolean) {
    setReviewerDraft((prev) => {
      const n = new Set(prev);
      if (on) n.add(userId);
      else n.delete(userId);
      return n;
    });
  }

  async function updateRole(userId: number, role: number) {
    const token = csrfToken ?? '';
    if (!token) return;
    setMemberErr(null);
    setMemberBusy(userId);
    try {
      await setProjectMemberRole(pid, userId, role, token);
      await reload();
    } catch (e) {
      setMemberErr(e instanceof Error ? e.message : 'Update failed');
    } finally {
      setMemberBusy(null);
    }
  }

  async function removeMember(userId: number) {
    const token = csrfToken ?? '';
    if (!token) return;
    if (!window.confirm(`Remove ${userLabel(userId)} from this project?`)) return;
    setMemberErr(null);
    setMemberBusy(userId);
    try {
      await removeProjectMember(pid, userId, token);
      await reload();
    } catch (e) {
      setMemberErr(e instanceof Error ? e.message : 'Remove failed');
    } finally {
      setMemberBusy(null);
    }
  }

  async function addMember(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    const uid = Number(addUserId);
    if (!token || !Number.isFinite(uid)) return;
    setMemberErr(null);
    setMemberBusy('add');
    try {
      await setProjectMemberRole(pid, uid, addRole, token);
      setAddUserId('');
      await reload();
    } catch (e) {
      setMemberErr(e instanceof Error ? e.message : 'Add failed');
    } finally {
      setMemberBusy(null);
    }
  }

  if (settings.loading || settings.error) return <SettingsLoadState settings={settings} />;

  return (
    <div>
      <section id="project-members" className="mb-10 scroll-mt-8">
        <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-4">
          Project members
        </h3>
        {memberErr && (
          <p className="text-sm text-red-300 mb-3">{memberErr}</p>
        )}
        {canManage && usersNotInProject.length > 0 && (
          <form
            onSubmit={addMember}
            className="mb-4 flex flex-wrap items-end gap-3 rounded-xl border border-stitch-border bg-stitch-elevated p-4"
          >
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
                Add user
              </label>
              <select
                value={addUserId}
                onChange={(e) => setAddUserId(e.target.value)}
                className="text-sm bg-stitch-surface border border-stitch-border rounded-md px-2 py-2 text-stitch-fg min-w-[200px]"
                required
              >
                <option value="">Select account…</option>
                {usersNotInProject.map((u) => (
                  <option key={u.id} value={u.id} className="bg-stitch-surface">
                    {u.name} ({u.username})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
                Role
              </label>
              <select
                value={addRole}
                onChange={(e) => setAddRole(Number(e.target.value))}
                className="text-sm bg-stitch-surface border border-stitch-border rounded-md px-2 py-2 text-stitch-fg"
              >
                {ROLES.map((r) => (
                  <option key={r.id} value={r.id} className="bg-stitch-surface">
                    {r.label}
                  </option>
                ))}
              </select>
            </div>
            <button
              type="submit"
              disabled={memberBusy === 'add'}
              className="bg-stitch-accent text-stitch-canvas text-xs font-bold uppercase px-4 py-2 rounded-md disabled:opacity-50"
            >
              {memberBusy === 'add' ? '…' : 'Add'}
            </button>
          </form>
        )}
        {canManage && users === null && (
          <p className="text-xs text-amber-700 dark:text-amber-200/90 mb-4">
            The user directory is visible to instance administrators only; ask one to add people to this project.
          </p>
        )}
        <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
                <th className="px-4 py-3">User</th>
                <th className="px-4 py-3">Role</th>
                {canManage ? <th className="px-4 py-3 text-right">Actions</th> : null}
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {members.map((m) => (
                <tr key={m.user_id} className="hover:bg-white/3">
                  <td className="px-4 py-3 text-stitch-fg">{userLabel(m.user_id)}</td>
                  <td className="px-4 py-3">
                    {canManage ? (
                      <select
                        value={m.role}
                        disabled={memberBusy === m.user_id}
                        onChange={(e) => void updateRole(m.user_id, Number(e.target.value))}
                        className="text-xs bg-stitch-elevated border border-stitch-border rounded-md px-2 py-1.5 text-stitch-fg"
                      >
                        {ROLES.map((r) => (
                          <option key={r.id} value={r.id} className="bg-stitch-surface">
                            {r.label}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <>
                        <span className="text-xs font-semibold text-stitch-accent">{m.role_label}</span>
                        <span className="text-[10px] text-stitch-muted ml-2">({m.role})</span>
                      </>
                    )}
                  </td>
                  {canManage ? (
                    <td className="px-4 py-3 text-right">
                      <button
                        type="button"
                        disabled={memberBusy === m.user_id}
                        onClick={() => void removeMember(m.user_id)}
                        className="text-xs font-bold text-red-300 hover:underline disabled:opacity-40"
                      >
                        Remove
                      </button>
                    </td>
                  ) : null}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {!canManage && (
          <p className="text-xs text-stitch-muted mt-3">
            You need “Manage members” to change roles here.
          </p>
        )}
      </section>

      <section id="project-reviewers" className="mb-10 scroll-mt-8">
        <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-2">
          Project reviewers
        </h3>
        <p className="text-xs text-stitch-muted mb-4 max-w-2xl">
          Only users checked here can change requirement and verification status and move requirement versions through
          draft → reviewed → approved. They must be project members. If the list is empty, only site administrators can
          perform those actions until you add at least one reviewer. Once the list is non-empty, being a site
          administrator does not bypass it—you must check yourself if you need those powers.
        </p>
        {reviewerErr && <p className="text-sm text-red-300 mb-3">{reviewerErr}</p>}
        <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch mb-4">
          <ul className="divide-y divide-stitch-border text-sm max-h-64 overflow-y-auto">
            {members.length === 0 ? (
              <li className="px-4 py-6 text-stitch-muted text-center">No members yet.</li>
            ) : (
              members.map((m) => (
                <li
                  key={m.user_id}
                  className="px-4 py-3 flex items-center justify-between gap-3 hover:bg-white/3"
                >
                  <span className="text-stitch-fg">{userLabel(m.user_id)}</span>
                  <div className="flex items-center gap-2 text-xs text-stitch-muted shrink-0">
                    <input
                      id={`project-reviewer-${m.user_id}`}
                      type="checkbox"
                      className="rounded-sm border-stitch-border cursor-pointer"
                      checked={reviewerDraft.has(m.user_id)}
                      disabled={!canManage || reviewerBusy}
                      onChange={(e) => toggleReviewerDraft(m.user_id, e.target.checked)}
                    />
                    <label
                      htmlFor={`project-reviewer-${m.user_id}`}
                      className={`cursor-pointer select-none ${!canManage || reviewerBusy ? 'opacity-50 cursor-not-allowed' : ''}`}
                    >
                      Reviewer
                    </label>
                  </div>
                </li>
              ))
            )}
          </ul>
        </div>
        {canManage ? (
          <div className="flex flex-wrap items-center gap-3">
            <button
              type="button"
              disabled={reviewerBusy || !reviewersDirty}
              onClick={() => void saveReviewers()}
              className="bg-stitch-accent text-stitch-canvas text-xs font-bold uppercase px-4 py-2 rounded-md disabled:opacity-50"
            >
              {reviewerBusy ? 'Saving…' : 'Save reviewer list'}
            </button>
            {reviewersDirty ? (
              <button
                type="button"
                disabled={reviewerBusy}
                className="text-xs text-stitch-muted hover:text-stitch-fg uppercase font-bold"
                onClick={() => setReviewerDraft(new Set(reviewerIds))}
              >
                Reset
              </button>
            ) : null}
          </div>
        ) : (
          <p className="text-xs text-stitch-muted">
            You need “Manage members” to edit the reviewer list.
          </p>
        )}
      </section>
    </div>
  );
}
