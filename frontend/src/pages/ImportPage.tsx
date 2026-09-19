import { FormEvent, useCallback, useMemo, useState } from 'react';
import { Link, useOutletContext } from 'react-router-dom';
import {
  commitExcelImport,
  getMyPermissions,
  listApplicability,
  listCategories,
  listProjectMembers,
  listRequirementStatuses,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  previewExcelImport,
} from '@/api/client';
import type {
  ExcelColumnMapping,
  ExcelImportPreview,
  ExcelImportResult,
  ExcelValueMapping,
} from '@/api/imports';
import type {
  Applicability,
  Category,
  ProjectMember,
  RequirementStatus,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { btnPrimary, inp } from '@/pages/catalog/catalogUi';
import { parseUser } from '@/utils/parseUser';

const FIELD_LABELS: Record<string, string> = {
  title: 'Title',
  description: 'Description',
  reference_code: 'Reference code',
  category_id: 'Category',
  applicability_id: 'Applicability',
  status_id: 'Status',
  verification_method_id: 'Verification method',
  author_id: 'Author',
  reviewer_id: 'Reviewer',
  parent_id: 'Parent',
  justification: 'Justification',
  name: 'Name',
  source: 'Source',
};

const VALUE_MAPPED_FIELDS = new Set([
  'category_id',
  'applicability_id',
  'status_id',
  'verification_method_id',
  'author_id',
  'reviewer_id',
]);

type CatalogData = {
  categories: Category[];
  applicability: Applicability[];
  requirementStatuses: RequirementStatus[];
  verificationStatuses: VerificationStatus[];
  methods: VerificationMethod[];
  members: ProjectMember[];
};

type ValueOption = {
  id: number;
  label: string;
  aliases: string[];
};

type UnmatchedValue = {
  targetField: string;
  sourceValue: string;
  options: ValueOption[];
  defaultId: number;
};

function normalized(value: string): string {
  return value.trim().toLowerCase();
}

function taggedOptions(
  rows: Array<{ id: number; title: string; tag: string }>,
): ValueOption[] {
  return rows
    .map((row) => ({
      id: row.id,
      label: row.title,
      aliases: [row.title, row.tag],
    }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

function suggestField(columnName: string, fields: string[]): string {
  const n = columnName.toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim();
  const aliases: Record<string, string> = {
    title: 'title',
    name: 'name',
    description: 'description',
    desc: 'description',
    reference: 'reference_code',
    'reference code': 'reference_code',
    'req id': 'reference_code',
    id: 'reference_code',
    category: 'category_id',
    applicability: 'applicability_id',
    status: 'status_id',
    verification: 'verification_method_id',
    'verification method': 'verification_method_id',
    author: 'author_id',
    reviewer: 'reviewer_id',
    parent: 'parent_id',
    justification: 'justification',
    source: 'source',
    'test name': 'name',
    'test id': 'reference_code',
  };
  const hinted = aliases[n];
  if (hinted && fields.includes(hinted)) return hinted;
  if (fields.includes(n.replace(/ /g, '_'))) return n.replace(/ /g, '_');
  return 'skip';
}

export default function ImportPage() {
  const { projectId: pid, basePath } = useOutletContext<ProjectOutletContext>();
  const { csrfToken, dashboard } = useDashboard();
  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<ExcelImportPreview | null>(null);
  const [importType, setImportType] = useState<'requirements' | 'tests'>('requirements');
  const [mappings, setMappings] = useState<Record<string, string>>({});
  const [catalog, setCatalog] = useState<CatalogData | null>(null);
  const [valueSelections, setValueSelections] = useState<Record<string, number>>({});
  const [busy, setBusy] = useState(false);
  const [phase, setPhase] = useState<'idle' | 'preview' | 'import'>('idle');
  const [err, setErr] = useState<string | null>(null);
  const [result, setResult] = useState<ExcelImportResult | null>(null);
  const [denied, setDenied] = useState(false);

  const token = csrfToken ?? '';
  const fields = useMemo(() => {
    if (!preview) return [];
    return importType === 'tests'
      ? preview.available_fields.tests
      : preview.available_fields.requirements;
  }, [preview, importType]);

  const valueOptions = useMemo<Record<string, ValueOption[]>>(() => {
    if (!catalog) return {} as Record<string, ValueOption[]>;
    const statuses =
      importType === 'tests'
        ? taggedOptions(catalog.verificationStatuses)
        : taggedOptions(catalog.requirementStatuses);
    const members = catalog.members
      .map((member) => ({
        id: member.user_id,
        label: member.name || member.username,
        aliases: [member.name, member.username],
      }))
      .sort((a, b) => a.label.localeCompare(b.label));
    return {
      category_id: taggedOptions(catalog.categories),
      applicability_id: taggedOptions(catalog.applicability),
      status_id: statuses,
      verification_method_id: taggedOptions(catalog.methods),
      author_id: members,
      reviewer_id: members,
    };
  }, [catalog, importType]);

  const unmatchedValues = useMemo<UnmatchedValue[]>(() => {
    if (!preview) return [];
    const currentUserId = parseUser(dashboard?.user)?.id;
    const seen = new Set<string>();
    const rows: UnmatchedValue[] = [];

    for (const column of preview.columns) {
      const targetField = mappings[column.name];
      if (!targetField || !VALUE_MAPPED_FIELDS.has(targetField)) continue;
      const options = valueOptions[targetField] ?? [];
      if (options.length === 0) continue;
      const preferred = options.find((option) => {
        const aliases = option.aliases.map(normalized);
        if (targetField === 'status_id' && importType === 'requirements') {
          return aliases.includes('draft') || aliases.includes('drf');
        }
        if (targetField === 'status_id' && importType === 'tests') {
          return aliases.includes('not run') || aliases.includes('nr');
        }
        if (targetField === 'author_id' || targetField === 'reviewer_id') {
          return option.id === currentUserId;
        }
        return false;
      });
      const defaultId = preferred?.id ?? Math.min(...options.map((option) => option.id));

      for (const sourceValue of preview.unique_values[column.name] ?? []) {
        const known = options.some((option) =>
          option.aliases.some((alias) => normalized(alias) === normalized(sourceValue)),
        );
        const key = `${targetField}\u0000${normalized(sourceValue)}`;
        if (!known && !seen.has(key)) {
          seen.add(key);
          rows.push({ targetField, sourceValue, options, defaultId });
        }
      }
    }
    return rows;
  }, [preview, mappings, valueOptions, importType, dashboard?.user]);

  const applyPreview = useCallback((p: ExcelImportPreview) => {
    const guessed = p.import_type === 'tests' ? 'tests' : 'requirements';
    const available =
      guessed === 'tests' ? p.available_fields.tests : p.available_fields.requirements;
    const next: Record<string, string> = {};
    for (const col of p.columns) {
      next[col.name] = suggestField(col.name, available);
    }
    setImportType(guessed);
    setMappings(next);
    setValueSelections({});
    setPreview(p);
    setResult(null);
  }, []);

  async function onUpload(e: FormEvent) {
    e.preventDefault();
    if (!file || !token) return;
    setBusy(true);
    setPhase('preview');
    setErr(null);
    setDenied(false);
    try {
      const perms = await getMyPermissions(pid).catch(() => null);
      if (!perms?.edit_requirements) {
        setDenied(true);
        return;
      }
      const [p, categories, applicability, requirementStatuses, verificationStatuses, methods, members] =
        await Promise.all([
          previewExcelImport(pid, file, token),
          listCategories(),
          listApplicability(),
          listRequirementStatuses(),
          listVerificationStatuses(),
          listVerificationMethodsByProject(pid),
          listProjectMembers(pid),
        ]);
      setCatalog({
        categories: categories.filter((item) => item.project_id === pid),
        applicability: applicability.filter((item) => item.project_id === pid),
        requirementStatuses: requirementStatuses.filter((item) => item.project_id === pid),
        verificationStatuses: verificationStatuses.filter((item) => item.project_id === pid),
        methods,
        members,
      });
      applyPreview(p);
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : 'Preview failed');
    } finally {
      setBusy(false);
      setPhase('idle');
    }
  }

  async function onCommit(e: FormEvent) {
    e.preventDefault();
    if (!file || !token || !preview) return;
    const columnMappings: ExcelColumnMapping[] = Object.entries(mappings).map(
      ([excel_column, target_field]) => ({ excel_column, target_field }),
    );
    const valueMappings: ExcelValueMapping[] = unmatchedValues.map((item) => ({
      target_field: item.targetField,
      source_value: item.sourceValue,
      target_id:
        valueSelections[`${item.targetField}\u0000${normalized(item.sourceValue)}`] ??
        item.defaultId,
    }));
    setBusy(true);
    setPhase('import');
    setErr(null);
    setResult(null);
    try {
      const r = await commitExcelImport(
        pid,
        file,
        importType,
        columnMappings,
        valueMappings,
        token,
      );
      setResult(r);
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : 'Import failed');
    } finally {
      setBusy(false);
      setPhase('idle');
    }
  }

  const listHref =
    importType === 'tests' ? `${basePath}/verifications` : `${basePath}/requirements`;

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Import"
        title="Import from Excel / CSV"
        subtitle="Upload a spreadsheet, map columns to Marreq fields, then create requirements or verifications."
      />

      {denied ? (
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-6 text-sm text-stitch-muted">
          You need permission to edit requirements to import files.
        </div>
      ) : null}

      {err ? (
        <div className="mb-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm p-4">
          {err}
        </div>
      ) : null}

      {phase !== 'idle' ? (
        <div
          className="mb-6 rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-3"
          data-testid="import-progress"
          role="status"
          aria-live="polite"
          aria-busy="true"
        >
          <div className="flex items-center gap-2">
            <span className="material-symbols-outlined text-stitch-accent text-xl animate-spin">
              progress_activity
            </span>
            <p className="text-sm font-semibold text-stitch-fg">
              {phase === 'preview'
                ? 'Reading file and preparing column mapping…'
                : `Importing ${preview?.row_count ?? 0} ${
                    importType === 'tests' ? 'verification' : 'requirement'
                  }${(preview?.row_count ?? 0) === 1 ? '' : 's'}…`}
            </p>
          </div>
          <div
            className="h-2 overflow-hidden rounded-full bg-stitch-elevated"
            role="progressbar"
            aria-label={phase === 'preview' ? 'Reading file' : 'Importing records'}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <div className="marreq-import-bar h-full w-1/3 rounded-full bg-stitch-accent" />
          </div>
          <p className="text-xs text-stitch-muted">
            {phase === 'import'
              ? 'This can take a while for large files. Keep this page open until it finishes.'
              : 'Please wait while the spreadsheet is parsed.'}
          </p>
        </div>
      ) : null}

      <form
        onSubmit={onUpload}
        className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-4 mb-6"
      >
        <label className="block text-xs font-bold uppercase tracking-wider text-stitch-muted">
          File
          <input
            type="file"
            accept=".csv,.xlsx,.xls,text/csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            className={`${inp} mt-2`}
            data-testid="import-file"
            disabled={busy}
            onChange={(e) => {
              setFile(e.target.files?.[0] ?? null);
              setPreview(null);
              setCatalog(null);
              setValueSelections({});
              setResult(null);
            }}
          />
        </label>
        <p className="text-xs text-stitch-muted">
          First sheet only. Use .xlsx or .csv. Unknown catalog names can be mapped to existing
          project values before import.
        </p>
        <button type="submit" disabled={!file || !token || busy} className={btnPrimary}>
          {phase === 'preview' ? 'Reading…' : 'Upload and map columns'}
        </button>
      </form>

      {preview ? (
        <form
          onSubmit={onCommit}
          className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-4 mb-6"
        >
          <div className="flex flex-wrap gap-4 items-end">
            <label className="text-xs font-bold uppercase tracking-wider text-stitch-muted">
              Import as
              <select
                className={`${inp} mt-2`}
                value={importType}
                disabled={busy}
                onChange={(e) => {
                  const next = e.target.value === 'tests' ? 'tests' : 'requirements';
                  setImportType(next);
                  const available =
                    next === 'tests'
                      ? preview.available_fields.tests
                      : preview.available_fields.requirements;
                  const remapped: Record<string, string> = {};
                  for (const col of preview.columns) {
                    remapped[col.name] = suggestField(col.name, available);
                  }
                  setMappings(remapped);
                  setValueSelections({});
                }}
              >
                <option value="requirements">Requirements</option>
                <option value="tests">Verifications</option>
              </select>
            </label>
            <p className="text-sm text-stitch-muted">
              {preview.row_count} data row{preview.row_count === 1 ? '' : 's'}
            </p>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full text-sm text-left">
              <thead>
                <tr className="text-[10px] uppercase tracking-wider text-stitch-muted border-b border-stitch-border">
                  <th className="py-2 pr-3">Column</th>
                  <th className="py-2 pr-3">Sample</th>
                  <th className="py-2">Marreq field</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-stitch-border">
                {preview.columns.map((col) => (
                  <tr key={col.name}>
                    <td className="py-2 pr-3 font-mono text-stitch-fg">{col.name}</td>
                    <td className="py-2 pr-3 text-stitch-muted max-w-xs truncate">
                      {col.sample_value || '—'}
                    </td>
                    <td className="py-2">
                      <select
                        className={inp}
                        value={mappings[col.name] ?? 'skip'}
                        disabled={busy}
                        onChange={(e) =>
                          setMappings((m) => ({ ...m, [col.name]: e.target.value }))
                        }
                        aria-label={`Map ${col.name}`}
                      >
                        <option value="skip">Skip</option>
                        {fields.map((f) => (
                          <option key={f} value={f}>
                            {FIELD_LABELS[f] ?? f}
                          </option>
                        ))}
                      </select>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {unmatchedValues.length > 0 ? (
            <div
              className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-4 space-y-3"
              data-testid="value-mapping"
            >
              <div>
                <h3 className="text-sm font-bold text-stitch-fg">Confirm unknown values</h3>
                <p className="text-xs text-stitch-muted mt-1">
                  These file values do not exist in this project. Defaults are selected; review
                  them before importing.
                </p>
              </div>
              <div className="space-y-2">
                {unmatchedValues.map((item) => {
                  const key = `${item.targetField}\u0000${normalized(item.sourceValue)}`;
                  return (
                    <label
                      key={key}
                      className="grid gap-2 md:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] md:items-center text-sm"
                    >
                      <span className="font-mono text-stitch-fg truncate">
                        {item.sourceValue}
                      </span>
                      <span className="text-stitch-muted">
                        {FIELD_LABELS[item.targetField] ?? item.targetField} →
                      </span>
                      <select
                        className={inp}
                        disabled={busy}
                        aria-label={`Map value ${item.sourceValue}`}
                        value={valueSelections[key] ?? item.defaultId}
                        onChange={(e) =>
                          setValueSelections((current) => ({
                            ...current,
                            [key]: Number(e.target.value),
                          }))
                        }
                      >
                        {item.options.map((option) => (
                          <option key={option.id} value={option.id}>
                            {option.label}
                          </option>
                        ))}
                      </select>
                    </label>
                  );
                })}
              </div>
            </div>
          ) : null}

          <button type="submit" disabled={busy || !token} className={btnPrimary}>
            {busy ? 'Importing…' : 'Import'}
          </button>
        </form>
      ) : null}

      {result ? (
        <div
          className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-3"
          data-testid="import-result"
        >
          <p className="text-stitch-fg font-semibold">{result.message}</p>
          <p className="text-sm text-stitch-muted">Imported {result.imported_count} record(s).</p>
          {result.errors.length > 0 ? (
            <ul className="text-sm text-red-300 list-disc pl-5 space-y-1">
              {result.errors.map((rowErr) => (
                <li key={rowErr}>{rowErr}</li>
              ))}
            </ul>
          ) : null}
          <Link to={listHref} className="text-stitch-accent font-semibold hover:underline text-sm">
            Open {importType === 'tests' ? 'verifications' : 'requirements'}
          </Link>
        </div>
      ) : null}
    </div>
  );
}
