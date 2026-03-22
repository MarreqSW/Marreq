import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import {
  getMyPermissions,
  listApplicability,
  listCategories,
  listCustomFieldsByProject,
  listMatrix,
  listProjectMembers,
  listRequirementStatuses,
  listRequirements,
  listUsersOptional,
  listVerificationMethodsByProject,
  listVerifications,
  patchRequirementByProject,
} from '@/api/client';
import type {
  Applicability,
  Category,
  CustomFieldDefinition,
  EffectivePermissions,
  MatrixLink,
  ProjectMember,
  Requirement,
  RequirementPatchBody,
  RequirementStatus,
  User,
  Verification,
  VerificationMethod,
} from '@/api/types';
import PriorityBadge from './PriorityBadge';
import { StatusBadge } from './StatusBadge';

function priorityFromCustomFields(req: Requirement): string {
  const fields = req.custom_fields;
  if (!fields?.length) return '—';
  const p = fields.find((f) => f.label && /priority/i.test(f.label));
  return p?.value?.trim() || '—';
}

function formatModified(iso: string): string {
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    const h = String(d.getHours()).padStart(2, '0');
    const min = String(d.getMinutes()).padStart(2, '0');
    return `${y}-${m}-${day} ${h}:${min}`;
  } catch {
    return iso;
  }
}

function ownerInitials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length >= 2) {
    return (parts[0]!.slice(0, 1) + parts[parts.length - 1]!.slice(0, 1)).toUpperCase();
  }
  return name.slice(0, 2).toUpperCase() || '?';
}

function escapeCsv(s: string): string {
  if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function approvalLabel(state: string): string {
  return state.replace(/_/g, ' ').toUpperCase();
}

function mergeRequirementAfterPatch(req: Requirement, patch: RequirementPatchBody): Requirement {
  const next: Requirement = { ...req };
  if (patch.title !== undefined) next.title = patch.title;
  if (patch.description !== undefined) next.description = patch.description;
  if (patch.status_id !== undefined) next.status_id = patch.status_id;
  if (patch.author_id !== undefined) next.author_id = patch.author_id;
  if (patch.reviewer_id !== undefined) next.reviewer_id = patch.reviewer_id;
  if (patch.category_id !== undefined) next.category_id = patch.category_id;
  if (patch.applicability_id !== undefined) next.applicability_id = patch.applicability_id;
  if (patch.verification_method_ids !== undefined) {
    next.verification_method_ids = [...patch.verification_method_ids];
  }
  if (patch.custom_fields?.length) {
    const byId = new Map((next.custom_fields ?? []).map((c) => [c.field_id, { ...c }]));
    for (const u of patch.custom_fields) {
      const cur = byId.get(u.field_id) ?? {
        field_id: u.field_id,
        label: '',
        value: null as string | null,
      };
      byId.set(u.field_id, { ...cur, value: u.value });
    }
    next.custom_fields = [...byId.values()];
  }
  return next;
}

/** Compact page list: 1 2 3 4 5 … 50 style */
function paginationItems(current: number, total: number): (number | 'dots')[] {
  if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
  if (current <= 4) return [1, 2, 3, 4, 5, 'dots', total];
  if (current >= total - 3) {
    return [1, 'dots', total - 4, total - 3, total - 2, total - 1, total];
  }
  return [1, 'dots', current - 1, current, current + 1, 'dots', total];
}

type ViewMode = 'table' | 'list';

export default function RequirementsTable({
  projectId,
  globalSearch,
  viewMode,
}: {
  projectId: number;
  globalSearch: string;
  viewMode: ViewMode;
}) {
  const navigate = useNavigate();
  const { dashboard, csrfToken } = useDashboard();
  const projectSlug = dashboard?.projects?.find((p) => p.id === projectId)?.slug;
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [statuses, setStatuses] = useState<RequirementStatus[]>([]);
  const [matrix, setMatrix] = useState<MatrixLink[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [applicability, setApplicability] = useState<Applicability[]>([]);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [verificationMethods, setVerificationMethods] = useState<VerificationMethod[]>([]);
  const [customFieldDefs, setCustomFieldDefs] = useState<CustomFieldDefinition[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [saveErr, setSaveErr] = useState<string | null>(null);
  const [savingId, setSavingId] = useState<number | null>(null);
  /** Last server-aligned row per id — avoid PATCH on blur when unchanged (reduces suspect churn). */
  const baselineRef = useRef<Map<number, Requirement>>(new Map());

  const [statusFilter, setStatusFilter] = useState<'all' | number>('all');
  const [priorityFilter, setPriorityFilter] = useState<'all' | 'P1' | 'P2' | 'P3'>('all');
  const [pageSize, setPageSize] = useState(25);
  const [page, setPage] = useState(1);
  const [selected, setSelected] = useState<Set<number>>(new Set());

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      setErr(null);
      try {
        const [
          reqs,
          st,
          mx,
          ver,
          u,
          cat,
          app,
          mem,
          methods,
          fields,
          permRes,
        ] = await Promise.all([
          listRequirements(projectId),
          listRequirementStatuses(),
          listMatrix(projectId),
          listVerifications(),
          listUsersOptional(),
          listCategories(),
          listApplicability(),
          listProjectMembers(projectId),
          listVerificationMethodsByProject(projectId),
          listCustomFieldsByProject(projectId),
          getMyPermissions(projectId).catch(() => null),
        ]);
        if (cancelled) return;
        baselineRef.current = new Map(reqs.map((r) => [r.id, { ...r }]));
        setRequirements(reqs);
        setStatuses(st);
        setMatrix(mx);
        setVerifications(ver.filter((v) => v.project_id === projectId));
        setUsers(u);
        setCategories(cat.filter((c) => c.project_id === projectId));
        setApplicability(app.filter((a) => a.project_id === projectId));
        setMembers(mem);
        setVerificationMethods(methods);
        setCustomFieldDefs([...fields].sort((a, b) => a.sort_order - b.sort_order));
        setPerms(permRes);
      } catch (e) {
        if (!cancelled) setErr(e instanceof Error ? e.message : 'Load failed');
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const statusById = useMemo(() => {
    const m = new Map<number, RequirementStatus>();
    for (const s of statuses) m.set(s.id, s);
    return m;
  }, [statuses]);

  const userById = useMemo(() => {
    const m = new Map<number, User>();
    if (users) for (const u of users) m.set(u.id, u);
    return m;
  }, [users]);

  const userLabel = useCallback(
    (id: number) => {
      const u = userById.get(id);
      if (u) return `${u.name} (${u.username})`;
      return `User #${id}`;
    },
    [userById],
  );

  const categoryById = useMemo(() => {
    const m = new Map<number, string>();
    for (const c of categories) m.set(c.id, c.title);
    return m;
  }, [categories]);

  const applicabilityById = useMemo(() => {
    const m = new Map<number, string>();
    for (const a of applicability) m.set(a.id, a.title);
    return m;
  }, [applicability]);

  const memberUserIds = useMemo(
    () => [...new Set(members.map((m) => m.user_id))].sort((a, b) => a - b),
    [members],
  );

  const reqTitleById = useMemo(() => {
    const m = new Map<number, string>();
    for (const r of requirements) m.set(r.id, r.title);
    return m;
  }, [requirements]);

  const canEdit = Boolean(perms?.edit_requirements && (csrfToken ?? '').length);

  const saveReq = useCallback(
    async (id: number, patch: RequirementPatchBody) => {
      const token = csrfToken ?? '';
      if (!token || !perms?.edit_requirements) return;
      setSaveErr(null);
      setSavingId(id);
      try {
        await patchRequirementByProject(projectId, id, patch, token);
        setRequirements((prev) =>
          prev.map((r) => (r.id === id ? mergeRequirementAfterPatch(r, patch) : r)),
        );
        const base = baselineRef.current.get(id);
        if (base) {
          baselineRef.current.set(id, mergeRequirementAfterPatch(base, patch));
        }
      } catch (e) {
        setSaveErr(e instanceof Error ? e.message : 'Save failed');
      } finally {
        setSavingId(null);
      }
    },
    [csrfToken, perms?.edit_requirements, projectId],
  );

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === projectId);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, projectId]);

  const linksPerReq = useMemo(() => {
    const m = new Map<number, number>();
    for (const row of matrix) {
      if (row.project_id !== projectId) continue;
      m.set(row.req_id, (m.get(row.req_id) ?? 0) + 1);
    }
    return m;
  }, [matrix, projectId]);

  const totalVerifications = Math.max(verifications.length, 1);

  const q = globalSearch.trim().toLowerCase();

  const filtered = useMemo(() => {
    return requirements.filter((req) => {
      if (statusFilter !== 'all' && req.status_id !== statusFilter) return false;
      const pr = priorityFromCustomFields(req);
      if (priorityFilter !== 'all') {
        const up = pr.toUpperCase();
        if (!up.includes(priorityFilter)) return false;
      }
      if (!q) return true;
      const blob = [
        req.reference_code,
        req.title,
        req.description,
        String(req.id),
      ]
        .join(' ')
        .toLowerCase();
      return blob.includes(q);
    });
  }, [requirements, statusFilter, priorityFilter, q]);

  useEffect(() => {
    setPage(1);
  }, [statusFilter, priorityFilter, q, pageSize]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const safePage = Math.min(page, pageCount);
  const sliceStart = (safePage - 1) * pageSize;
  const pageRows = useMemo(
    () => filtered.slice(sliceStart, sliceStart + pageSize),
    [filtered, sliceStart, pageSize],
  );

  const resetFilters = useCallback(() => {
    setStatusFilter('all');
    setPriorityFilter('all');
  }, []);

  const toggleAllPage = useCallback(() => {
    const ids = pageRows.map((r) => r.id);
    const allOn = ids.length > 0 && ids.every((id) => selected.has(id));
    setSelected((prev) => {
      const next = new Set(prev);
      if (allOn) for (const id of ids) next.delete(id);
      else for (const id of ids) next.add(id);
      return next;
    });
  }, [pageRows, selected]);

  const exportCsv = useCallback(() => {
    const headers = [
      'Key',
      'Title',
      'Category',
      'Parent id',
      'Status',
      'Approval',
      'Verification method ids',
      'Modified',
      'Author',
      'Reviewer',
      'Applicability',
      'Description',
      'Priority',
      'Coverage %',
    ];
    const lines = [headers.join(',')];
    for (const req of filtered) {
      const st = statusById.get(req.status_id);
      const linked = linksPerReq.get(req.id) ?? 0;
      const coverage = Math.min(100, Math.round((linked / totalVerifications) * 100));
      lines.push(
        [
          escapeCsv(req.reference_code || `#${req.id}`),
          escapeCsv(req.title),
          escapeCsv(categoryById.get(req.category_id) ?? ''),
          escapeCsv(req.parent_id != null ? String(req.parent_id) : ''),
          escapeCsv(st?.title ?? ''),
          escapeCsv(req.approval_state),
          escapeCsv((req.verification_method_ids ?? []).join(';')),
          escapeCsv(formatModified(req.update_date)),
          escapeCsv(userLabel(req.author_id)),
          escapeCsv(userLabel(req.reviewer_id)),
          escapeCsv(applicabilityById.get(req.applicability_id) ?? ''),
          escapeCsv(req.description?.replace(/\s+/g, ' ').slice(0, 500) ?? ''),
          escapeCsv(priorityFromCustomFields(req)),
          escapeCsv(String(coverage)),
        ].join(','),
      );
    }
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `requirements-project-${projectId}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }, [
    filtered,
    statusById,
    categoryById,
    applicabilityById,
    userLabel,
    linksPerReq,
    totalVerifications,
    projectId,
  ]);

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted font-mono italic border border-stitch-border rounded-xl bg-stitch-surface">
        Loading registry…
      </div>
    );
  }

  if (err) {
    return (
      <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm">
        {err}
      </div>
    );
  }

  const pageIds = pageRows.map((r) => r.id);
  const allPageSelected =
    pageIds.length > 0 && pageIds.every((id) => selected.has(id));

  const cellInput =
    'w-full min-w-[90px] max-w-[min(100%,280px)] text-xs bg-stitch-elevated border border-stitch-border rounded px-2 py-1.5 text-white focus:border-stitch-accent outline-none disabled:opacity-50';
  const cellSelect = `${cellInput} cursor-pointer`;

  return (
    <div className="space-y-6">
      {saveErr ? (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 text-red-100 text-sm px-4 py-2">
          {saveErr}
        </div>
      ) : null}
      {/* Filters bar — Image 2.html */}
      <div className="bg-stitch-elevated p-4 rounded-xl border border-stitch-border flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-2 md:gap-3">
          <div className="flex items-center gap-2 px-3 py-1.5 bg-stitch-surface border border-stitch-border rounded text-xs text-stitch-muted">
            <span className="material-symbols-outlined text-sm">filter_list</span>
            <span>Status:</span>
            <select
              value={statusFilter === 'all' ? 'all' : String(statusFilter)}
              onChange={(e) => {
                const v = e.target.value;
                setStatusFilter(v === 'all' ? 'all' : Number(v));
              }}
              className="bg-transparent text-stitch-accent font-bold text-xs border-none outline-none cursor-pointer"
            >
              <option value="all">All</option>
              {statusOptions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.title}
                </option>
              ))}
            </select>
          </div>
          <div className="flex items-center gap-2 px-3 py-1.5 bg-stitch-surface border border-stitch-border rounded text-xs text-stitch-muted">
            <span>Priority:</span>
            <select
              value={priorityFilter}
              onChange={(e) => setPriorityFilter(e.target.value as typeof priorityFilter)}
              className="bg-transparent text-stitch-accent font-bold text-xs border-none outline-none cursor-pointer"
            >
              <option value="all">All</option>
              <option value="P1">P1</option>
              <option value="P2">P2</option>
              <option value="P3">P3</option>
            </select>
          </div>
          <div className="hidden sm:block h-4 w-px bg-stitch-border mx-1" />
          <button
            type="button"
            onClick={resetFilters}
            className="text-xs text-stitch-accent font-bold flex items-center gap-1 hover:underline"
          >
            <span className="material-symbols-outlined text-sm">restart_alt</span>
            Reset Filters
          </button>
        </div>
        <div className="flex items-center gap-4">
          <p className="text-[10px] text-stitch-muted font-mono">
            {filtered.length.toLocaleString()} Requirements found
          </p>
          <button
            type="button"
            onClick={exportCsv}
            title="Download CSV"
            className="p-2 text-stitch-muted hover:text-stitch-accent transition-colors"
          >
            <span className="material-symbols-outlined">file_download</span>
          </button>
          {projectSlug ? (
            <a
              href={`/p/${projectSlug}/requirements.xls`}
              title="Download Excel (classic)"
              className="p-2 text-stitch-muted hover:text-stitch-accent transition-colors"
            >
              <span className="material-symbols-outlined">table_chart</span>
            </a>
          ) : null}
        </div>
      </div>

      {selected.size > 0 && (
        <div className="flex flex-wrap items-center justify-between gap-3 bg-[#1a237e]/90 text-white px-4 py-3 rounded-lg border border-stitch-accent/30 shadow-xl">
          <div className="flex flex-wrap items-center gap-3 text-sm">
            <span className="font-bold">{selected.size} selected</span>
            <div className="h-4 w-px bg-white/20 hidden sm:block" />
            <span className="text-xs text-white/70">Bulk actions coming soon</span>
          </div>
          <button
            type="button"
            className="text-xs opacity-80 hover:opacity-100"
            onClick={() => setSelected(new Set())}
            aria-label="Clear selection"
          >
            <span className="material-symbols-outlined">close</span>
          </button>
        </div>
      )}

      {viewMode === 'list' ? (
        <ul className="space-y-3">
          {pageRows.map((req) => {
            const st = statusById.get(req.status_id);
            const statusTitle = st?.title ?? `Status #${req.status_id}`;
            const u = userById.get(req.author_id);
            const ownerName = u?.name ?? u?.username ?? `User #${req.author_id}`;
            const linked = linksPerReq.get(req.id) ?? 0;
            const coverage = Math.min(100, Math.round((linked / totalVerifications) * 100));
            return (
              <li
                key={req.id}
                className="rounded-xl border border-stitch-border bg-stitch-surface p-4 shadow-stitch hover:bg-white/[0.03] transition-colors"
              >
                <div className="flex items-start gap-3">
                  <input
                    type="checkbox"
                    className="rounded border-stitch-border mt-1 w-4 h-4 text-stitch-accent focus:ring-stitch-accent"
                    checked={selected.has(req.id)}
                    onChange={(e) => {
                      e.stopPropagation();
                      setSelected((prev) => {
                        const next = new Set(prev);
                        if (e.target.checked) next.add(req.id);
                        else next.delete(req.id);
                        return next;
                      });
                    }}
                    onClick={(e) => e.stopPropagation()}
                  />
                  <div className="flex-1 min-w-0">
                    <div className="flex flex-wrap items-center gap-2 mb-1">
                      <span className="font-mono text-sm text-stitch-accent font-semibold">
                        {req.reference_code || `#${req.id}`}
                      </span>
                      <StatusBadge title={statusTitle} />
                      <PriorityBadge value={priorityFromCustomFields(req)} />
                      <span className="text-[10px] uppercase text-stitch-muted border border-stitch-border rounded px-1.5 py-0.5">
                        {approvalLabel(req.approval_state)}
                      </span>
                      <span className="text-[10px] text-stitch-muted">
                        {categoryById.get(req.category_id) ?? `Cat #${req.category_id}`}
                      </span>
                    </div>
                    <p className="text-sm font-semibold text-white">{req.title}</p>
                    <p className="text-xs text-stitch-muted mt-0.5 line-clamp-2">{req.description}</p>
                    <div className="flex flex-wrap items-center gap-4 mt-3 text-xs text-stitch-muted">
                      <div className="flex items-center gap-2">
                        <div className="w-6 h-6 rounded-full bg-stitch-elevated flex items-center justify-center text-[10px] font-bold text-white border border-stitch-border">
                          {ownerInitials(ownerName)}
                        </div>
                        <span className="text-white/90 font-medium">{ownerName}</span>
                      </div>
                      <span className="font-mono">{formatModified(req.update_date)}</span>
                      <span>Coverage {coverage}%</span>
                      <button
                        type="button"
                        className="text-stitch-accent font-bold hover:underline"
                        onClick={() => navigate(`/p/${projectId}/requirements/${req.id}/edit`)}
                      >
                        Open editor
                      </button>
                    </div>
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      ) : (
        <div className="bg-stitch-surface overflow-x-auto rounded-xl border border-stitch-border shadow-stitch">
          <table className="w-full text-left border-collapse min-w-[1400px]">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated">
                <th className="p-3 w-10 sticky left-0 z-10 bg-stitch-elevated">
                  <input
                    type="checkbox"
                    className="rounded border-stitch-border w-4 h-4 text-stitch-accent focus:ring-stitch-accent"
                    checked={allPageSelected}
                    onChange={toggleAllPage}
                    aria-label="Select page"
                  />
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider whitespace-nowrap">
                  Key
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                  Title
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Category
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Parent
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[110px]">
                  Status
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider">
                  Approval
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                  Verification methods
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider whitespace-nowrap">
                  Modified
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Author
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Reviewer
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Applicability
                </th>
                {customFieldDefs.map((def) => (
                  <th
                    key={def.id}
                    className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[100px] max-w-[160px]"
                  >
                    {def.label}
                  </th>
                ))}
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[180px]">
                  Statement
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider text-center">
                  Pri.
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[100px]">
                  Coverage
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider sticky right-0 bg-stitch-elevated min-w-[88px]">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {pageRows.map((req) => {
                const linked = linksPerReq.get(req.id) ?? 0;
                const coverage = Math.min(100, Math.round((linked / totalVerifications) * 100));
                const methodIds = req.verification_method_ids ?? [];
                const busy = savingId === req.id;
                return (
                  <tr key={req.id} className="hover:bg-white/[0.03] transition-colors">
                    <td className="p-3 sticky left-0 z-[1] bg-stitch-surface border-r border-stitch-border/60">
                      <input
                        type="checkbox"
                        className="rounded border-stitch-border w-4 h-4 text-stitch-accent focus:ring-stitch-accent"
                        checked={selected.has(req.id)}
                        onChange={(e) => {
                          setSelected((prev) => {
                            const next = new Set(prev);
                            if (e.target.checked) next.add(req.id);
                            else next.delete(req.id);
                            return next;
                          });
                        }}
                      />
                    </td>
                    <td className="px-2 py-2 align-top">
                      <span className="text-xs font-mono text-stitch-accent font-semibold whitespace-nowrap">
                        {req.reference_code || `REQ-${req.id}`}
                      </span>
                    </td>
                    <td className="px-2 py-2 align-top">
                      <input
                        className={cellInput}
                        value={req.title}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const v = e.target.value;
                          setRequirements((prev) =>
                            prev.map((r) => (r.id === req.id ? { ...r, title: v } : r)),
                          );
                        }}
                        onBlur={(e) => {
                          const v = e.target.value.trim();
                          const b = baselineRef.current.get(req.id);
                          if (b && v === b.title) return;
                          void saveReq(req.id, { title: v });
                        }}
                      />
                    </td>
                    <td className="px-2 py-2 align-top">
                      <select
                        className={cellSelect}
                        value={req.category_id}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          setRequirements((prev) =>
                            prev.map((r) => (r.id === req.id ? { ...r, category_id: v } : r)),
                          );
                          void saveReq(req.id, { category_id: v });
                        }}
                      >
                        {categories.map((c) => (
                          <option key={c.id} value={c.id} className="bg-stitch-surface">
                            {c.title}
                          </option>
                        ))}
                        {!categories.some((c) => c.id === req.category_id) && (
                          <option value={req.category_id} className="bg-stitch-surface">
                            Category #{req.category_id}
                          </option>
                        )}
                      </select>
                    </td>
                    <td className="px-2 py-2 align-top text-xs">
                      {req.parent_id != null ? (
                        <Link
                          to={`/p/${projectId}/requirements/${req.parent_id}/edit`}
                          className="text-stitch-accent hover:underline line-clamp-2"
                        >
                          {reqTitleById.get(req.parent_id) ?? `REQ #${req.parent_id}`}
                        </Link>
                      ) : (
                        <span className="text-stitch-muted">—</span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top">
                      <select
                        className={cellSelect}
                        value={req.status_id}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          setRequirements((prev) =>
                            prev.map((r) => (r.id === req.id ? { ...r, status_id: v } : r)),
                          );
                          void saveReq(req.id, { status_id: v });
                        }}
                      >
                        {statusOptions.map((s) => (
                          <option key={s.id} value={s.id} className="bg-stitch-surface">
                            {s.title}
                          </option>
                        ))}
                        {!statusOptions.some((s) => s.id === req.status_id) && (
                          <option value={req.status_id} className="bg-stitch-surface">
                            Status #{req.status_id}
                          </option>
                        )}
                      </select>
                    </td>
                    <td className="px-2 py-2 align-top">
                      <span className="text-[10px] font-bold uppercase text-stitch-muted border border-stitch-border rounded px-1.5 py-1 inline-block max-w-[120px] truncate">
                        {approvalLabel(req.approval_state)}
                      </span>
                    </td>
                    <td className="px-2 py-2 align-top">
                      <select
                        multiple
                        size={Math.min(4, Math.max(2, verificationMethods.length || 2))}
                        className={`${cellInput} min-h-[48px]`}
                        value={methodIds.map(String)}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const sel = [...e.target.selectedOptions].map((o) => Number(o.value));
                          setRequirements((prev) =>
                            prev.map((r) =>
                              r.id === req.id ? { ...r, verification_method_ids: sel } : r,
                            ),
                          );
                          void saveReq(req.id, { verification_method_ids: sel });
                        }}
                      >
                        {verificationMethods.map((m) => (
                          <option key={m.id} value={m.id} className="bg-stitch-surface">
                            {m.title}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="px-2 py-2 align-top text-[11px] text-stitch-muted font-mono whitespace-nowrap">
                      {formatModified(req.update_date)}
                    </td>
                    <td className="px-2 py-2 align-top">
                      <select
                        className={cellSelect}
                        value={req.author_id}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          setRequirements((prev) =>
                            prev.map((r) => (r.id === req.id ? { ...r, author_id: v } : r)),
                          );
                          void saveReq(req.id, { author_id: v });
                        }}
                      >
                        {memberUserIds.map((id) => (
                          <option key={id} value={id} className="bg-stitch-surface">
                            {userLabel(id)}
                          </option>
                        ))}
                        {!memberUserIds.includes(req.author_id) && (
                          <option value={req.author_id} className="bg-stitch-surface">
                            {userLabel(req.author_id)}
                          </option>
                        )}
                      </select>
                    </td>
                    <td className="px-2 py-2 align-top">
                      <select
                        className={cellSelect}
                        value={req.reviewer_id}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          setRequirements((prev) =>
                            prev.map((r) => (r.id === req.id ? { ...r, reviewer_id: v } : r)),
                          );
                          void saveReq(req.id, { reviewer_id: v });
                        }}
                      >
                        {memberUserIds.map((id) => (
                          <option key={`rev-${id}`} value={id} className="bg-stitch-surface">
                            {userLabel(id)}
                          </option>
                        ))}
                        {!memberUserIds.includes(req.reviewer_id) && (
                          <option value={req.reviewer_id} className="bg-stitch-surface">
                            {userLabel(req.reviewer_id)}
                          </option>
                        )}
                      </select>
                    </td>
                    <td className="px-2 py-2 align-top">
                      <select
                        className={cellSelect}
                        value={req.applicability_id}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const v = Number(e.target.value);
                          setRequirements((prev) =>
                            prev.map((r) => (r.id === req.id ? { ...r, applicability_id: v } : r)),
                          );
                          void saveReq(req.id, { applicability_id: v });
                        }}
                      >
                        {applicability.map((a) => (
                          <option key={a.id} value={a.id} className="bg-stitch-surface">
                            {a.title}
                          </option>
                        ))}
                        {!applicability.some((a) => a.id === req.applicability_id) && (
                          <option value={req.applicability_id} className="bg-stitch-surface">
                            Applicability #{req.applicability_id}
                          </option>
                        )}
                      </select>
                    </td>
                    {customFieldDefs.map((def) => {
                      const val =
                        req.custom_fields?.find((c) => c.field_id === def.id)?.value ?? '';
                      return (
                        <td key={def.id} className="px-2 py-2 align-top">
                          <input
                            className={cellInput}
                            value={val}
                            disabled={!canEdit || busy}
                            onChange={(e) => {
                              const v = e.target.value;
                              setRequirements((prev) =>
                                prev.map((r) => {
                                  if (r.id !== req.id) return r;
                                  const cfs = [...(r.custom_fields ?? [])];
                                  const ix = cfs.findIndex((c) => c.field_id === def.id);
                                  if (ix >= 0) cfs[ix] = { ...cfs[ix]!, value: v };
                                  else cfs.push({ field_id: def.id, label: def.label, value: v });
                                  return { ...r, custom_fields: cfs };
                                }),
                              );
                            }}
                            onBlur={(e) => {
                              const v = e.target.value.trim() || null;
                              const b = baselineRef.current.get(req.id);
                              const prev =
                                b?.custom_fields?.find((c) => c.field_id === def.id)?.value ?? null;
                              if (prev === v || (prev === '' && v === null)) return;
                              void saveReq(req.id, {
                                custom_fields: [{ field_id: def.id, value: v }],
                              });
                            }}
                          />
                        </td>
                      );
                    })}
                    <td className="px-2 py-2 align-top">
                      <textarea
                        className={`${cellInput} min-h-[64px] resize-y max-w-[320px]`}
                        rows={3}
                        value={req.description}
                        disabled={!canEdit || busy}
                        onChange={(e) => {
                          const v = e.target.value;
                          setRequirements((prev) =>
                            prev.map((r) => (r.id === req.id ? { ...r, description: v } : r)),
                          );
                        }}
                        onBlur={(e) => {
                          const v = e.target.value.trim();
                          const b = baselineRef.current.get(req.id);
                          if (b && v === b.description) return;
                          void saveReq(req.id, { description: v });
                        }}
                      />
                    </td>
                    <td className="px-2 py-2 align-top text-center">
                      <PriorityBadge value={priorityFromCustomFields(req)} />
                    </td>
                    <td className="px-2 py-2 align-top">
                      <div className="flex items-center gap-2">
                        <div className="w-14 bg-white/10 h-1.5 rounded-full overflow-hidden shrink-0">
                          <div
                            className="bg-stitch-accent h-full transition-all rounded-full"
                            style={{ width: `${coverage}%` }}
                          />
                        </div>
                        <span className="text-[11px] text-stitch-muted font-mono tabular-nums">
                          {coverage}%
                        </span>
                      </div>
                    </td>
                    <td className="px-2 py-2 align-top sticky right-0 z-[1] bg-stitch-surface border-l border-stitch-border/60">
                      <div className="flex items-center gap-1">
                        {projectSlug ? (
                          <a
                            href={`/p/${projectSlug}/requirements/show/${req.id}`}
                            className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                            title="Classic view"
                          >
                            <span className="material-symbols-outlined text-lg">visibility</span>
                          </a>
                        ) : null}
                        <Link
                          to={`/p/${projectId}/requirements/${req.id}/edit`}
                          className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                          title="SPA editor"
                        >
                          <span className="material-symbols-outlined text-lg">edit</span>
                        </Link>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {filtered.length === 0 && (
            <p className="p-6 text-center text-stitch-muted text-sm">No requirements match filters.</p>
          )}
        </div>
      )}

      {filtered.length > 0 && (
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-2 text-xs text-stitch-muted">
            <span>Show rows:</span>
            <select
              value={pageSize}
              onChange={(e) => setPageSize(Number(e.target.value))}
              className="bg-stitch-elevated border border-stitch-border rounded text-xs py-1 px-2 text-white focus:ring-1 focus:ring-stitch-accent outline-none"
            >
              <option value={25}>25</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
            </select>
          </div>
          <div className="flex flex-wrap items-center gap-4">
            <p className="text-xs text-stitch-muted font-medium">
              Showing{' '}
              <span className="text-white font-bold">
                {filtered.length === 0 ? 0 : sliceStart + 1}-{Math.min(sliceStart + pageSize, filtered.length)}
              </span>{' '}
              of <span className="text-white font-bold">{filtered.length}</span>
            </p>
            <div className="flex items-center gap-1">
              <button
                type="button"
                disabled={safePage <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                className="p-1 text-stitch-muted hover:text-stitch-accent transition-colors disabled:opacity-30"
              >
                <span className="material-symbols-outlined">chevron_left</span>
              </button>
              <div className="flex gap-1 items-center">
                {paginationItems(safePage, pageCount).map((item, idx) =>
                  item === 'dots' ? (
                    <span key={`dots-${idx}`} className="px-1 text-stitch-muted text-xs">
                      …
                    </span>
                  ) : (
                    <button
                      key={item}
                      type="button"
                      onClick={() => setPage(item)}
                      className={`w-8 h-8 flex items-center justify-center rounded text-xs font-bold transition-colors ${
                        item === safePage
                          ? 'bg-stitch-accent text-stitch-canvas'
                          : 'hover:bg-white/[0.08] text-white'
                      }`}
                    >
                      {item}
                    </button>
                  ),
                )}
              </div>
              <button
                type="button"
                disabled={safePage >= pageCount}
                onClick={() => setPage((p) => Math.min(pageCount, p + 1))}
                className="p-1 text-stitch-muted hover:text-stitch-accent transition-colors disabled:opacity-30"
              >
                <span className="material-symbols-outlined">chevron_right</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
