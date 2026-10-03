import { type FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useOutletContext, useSearchParams } from 'react-router-dom';
import {
  createReportTemplate,
  deleteReportTemplate,
  downloadReport,
  fetchReport,
  listReportTemplates,
  listReportTypes,
  updateReportTemplate,
} from '@/api/client';
import type {
  ReportDefinition,
  ReportFormat,
  ReportTemplate,
  ReportTypeInfo,
  ReportTypeKey,
} from '@/api/reports';
import Dialog from '@/components/Dialog';
import DocumentSettingsForm from '@/components/reports/DocumentSettingsForm';
import SectionList from '@/components/reports/SectionList';
import StitchPageHeader from '@/components/StitchPageHeader';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { cloneDefinition, sameDefinition } from '@/utils/reportDefinition';

const btn =
  'rounded-md border border-stitch-border px-3 py-1.5 text-xs font-semibold text-stitch-fg hover:bg-stitch-elevated disabled:opacity-50';
const btnPrimary =
  'rounded-md bg-stitch-accent px-3 py-1.5 text-xs font-semibold text-white hover:opacity-90 disabled:opacity-50';
const selectCls = 'rounded-md border border-stitch-border bg-stitch-surface px-2 py-1.5 text-sm';

/**
 * Reports › Customize (issue #355): choose a report document's sections,
 * their order and options, and its document fields; save the result as a
 * template; preview or download it as PDF or ODT.
 */
export default function ReportBuilderPage() {
  const { projectId: pid, basePath } = useOutletContext<ProjectOutletContext>();
  const { csrfToken, dashboard } = useDashboard();
  const projectName = dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';
  const token = csrfToken ?? '';
  const [params, setParams] = useSearchParams();
  const typeKey = (params.get('type') === 'coverage' ? 'coverage' : 'vcd') as ReportTypeKey;
  const templateParam = params.get('template');
  const templateId = templateParam ? Number(templateParam) : null;

  const [types, setTypes] = useState<ReportTypeInfo[] | null>(null);
  const [templates, setTemplates] = useState<ReportTemplate[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [draft, setDraft] = useState<ReportDefinition | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [saveAsOpen, setSaveAsOpen] = useState(false);
  const [saveAsName, setSaveAsName] = useState('');
  const [saveAsShared, setSaveAsShared] = useState(false);

  useEffect(() => {
    let cancelled = false;
    Promise.all([listReportTypes(pid), listReportTemplates(pid)])
      .then(([t, tpl]) => {
        if (cancelled) return;
        setTypes(t);
        setTemplates(tpl);
      })
      .catch((e: unknown) => {
        if (!cancelled) setLoadError(e instanceof Error ? e.message : 'Could not load the report types.');
      });
    return () => {
      cancelled = true;
    };
  }, [pid]);

  const info = types?.find((t) => t.key === typeKey) ?? null;
  const template = templates.find((t) => t.id === templateId && t.report_type === typeKey) ?? null;
  const base = template?.definition ?? info?.default_definition ?? null;
  const typeTemplates = templates.filter((t) => t.report_type === typeKey);

  // Start from the selected template (or the default) whenever it changes.
  useEffect(() => {
    setDraft(base ? cloneDefinition(base) : null);
    // `base` is a new object on every template list refresh; key on what it is.
  }, [typeKey, template?.id, template?.updated_at, info?.key]);

  const dirty = draft != null && !sameDefinition(draft, base);

  useEffect(() => {
    if (!dirty) return;
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      e.preventDefault();
    };
    window.addEventListener('beforeunload', onBeforeUnload);
    return () => window.removeEventListener('beforeunload', onBeforeUnload);
  }, [dirty]);

  const confirmDiscard = () => !dirty || window.confirm('Discard your unsaved changes?');

  const select = (next: { type?: ReportTypeKey; template?: number | null }) => {
    if (!confirmDiscard()) return;
    const sp = new URLSearchParams();
    sp.set('type', next.type ?? typeKey);
    const tpl = next.template === undefined ? templateId : next.template;
    if (tpl != null && next.type === undefined) sp.set('template', String(tpl));
    setNotice(null);
    setError(null);
    setParams(sp, { replace: true });
  };

  const run = useCallback(async (label: string, action: () => Promise<void>) => {
    setBusy(label);
    setError(null);
    setNotice(null);
    try {
      await action();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Something went wrong.');
    } finally {
      setBusy(null);
    }
  }, []);

  const refreshTemplates = async () => setTemplates(await listReportTemplates(pid));

  const generate = (format: ReportFormat) =>
    run(format, async () => {
      if (!draft) return;
      await downloadReport(pid, typeKey, format, { definition: draft }, token);
    });

  const preview = () => {
    if (!draft) return;
    // Open the tab now: browsers block windows opened after an await.
    const tab = window.open('', '_blank');
    void run('preview', async () => {
      try {
        const { blob } = await fetchReport(pid, typeKey, 'pdf', { definition: draft }, token);
        const url = URL.createObjectURL(blob);
        if (tab) tab.location.href = url;
        else window.location.assign(url);
        window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
      } catch (e) {
        tab?.close();
        throw e;
      }
    });
  };

  const save = () =>
    run('save', async () => {
      if (!template || !draft) return;
      await updateReportTemplate(pid, template.id, { definition: draft }, token);
      await refreshTemplates();
      setNotice(`Saved "${template.name}".`);
    });

  async function saveAs(e: FormEvent) {
    e.preventDefault();
    if (!draft || saveAsName.trim() === '') return;
    await run('save-as', async () => {
      const created = await createReportTemplate(
        pid,
        { name: saveAsName.trim(), visibility: saveAsShared ? 'shared' : 'private', definition: draft },
        token,
      );
      await refreshTemplates();
      setSaveAsOpen(false);
      setParams(new URLSearchParams({ type: typeKey, template: String(created.id) }), { replace: true });
      setNotice(`Saved as "${created.name}".`);
    });
  }

  const remove = () => {
    if (!template || !window.confirm(`Delete the template "${template.name}"?`)) return;
    void run('delete', async () => {
      await deleteReportTemplate(pid, template.id, token);
      await refreshTemplates();
      setParams(new URLSearchParams({ type: typeKey }), { replace: true });
      setNotice(`Deleted "${template.name}".`);
    });
  };

  const enabledCount = useMemo(() => draft?.sections.filter((s) => s.enabled).length ?? 0, [draft]);

  if (loadError) {
    return (
      <p role="alert" className="text-sm text-red-700 dark:text-red-300">
        {loadError}
      </p>
    );
  }
  if (!types || !info || !draft) {
    return <p className="text-sm text-stitch-muted">Loading…</p>;
  }

  return (
    <div className="max-w-6xl">
      <StitchPageHeader
        projectName={projectName}
        section="Reports"
        title="Customize report"
        subtitle="Choose the sections, their order and the document fields, then save the template or download the document."
      />
      <p className="mb-4 text-xs">
        <Link to={`${basePath}/reports`} className="text-stitch-accent hover:underline">
          ← Reports & exports
        </Link>
      </p>

      <div className="mb-6 flex flex-wrap items-end gap-3 rounded-xl border border-stitch-border bg-stitch-surface p-4">
        <label className="block">
          <span className="block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1">Report</span>
          <select
            className={selectCls}
            value={typeKey}
            onChange={(e) => select({ type: e.target.value as ReportTypeKey })}
          >
            {types.map((t) => (
              <option key={t.key} value={t.key}>
                {t.title}
              </option>
            ))}
          </select>
        </label>
        <label className="block">
          <span className="block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1">Template</span>
          <select
            className={selectCls}
            value={template ? String(template.id) : ''}
            onChange={(e) => select({ template: e.target.value === '' ? null : Number(e.target.value) })}
          >
            <option value="">Default</option>
            {typeTemplates.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
                {t.visibility === 'shared' ? ' (shared)' : ''}
              </option>
            ))}
          </select>
        </label>
        <div className="flex flex-wrap gap-2 ml-auto">
          {template ? (
            <button
              type="button"
              className={btn}
              disabled={!template.can_edit || !dirty || busy !== null}
              title={template.can_edit ? undefined : `Only ${template.owner_name} or a project admin can change it`}
              onClick={() => void save()}
            >
              {busy === 'save' ? 'Saving…' : 'Save'}
            </button>
          ) : null}
          <button
            type="button"
            className={btn}
            disabled={busy !== null}
            onClick={() => {
              setSaveAsName(template ? `${template.name} (copy)` : '');
              setSaveAsShared(false);
              setSaveAsOpen(true);
            }}
          >
            Save as…
          </button>
          {template?.can_edit ? (
            <button type="button" className={btn} disabled={busy !== null} onClick={remove}>
              Delete
            </button>
          ) : null}
          <button
            type="button"
            className={btn}
            disabled={busy !== null || sameDefinition(draft, info.default_definition)}
            onClick={() => setDraft(cloneDefinition(info.default_definition))}
          >
            Reset to default
          </button>
        </div>
      </div>

      {template && !template.can_edit ? (
        <p className="mb-4 text-xs text-stitch-muted">
          This template belongs to {template.owner_name}. You can change it here and download the result, or save
          your own copy with <strong>Save as…</strong>.
        </p>
      ) : null}
      {dirty ? <p className="mb-4 text-xs text-amber-700 dark:text-amber-300">Unsaved changes.</p> : null}
      {error ? (
        <p role="alert" className="mb-4 text-sm text-red-700 dark:text-red-300">
          {error}
        </p>
      ) : null}
      {notice ? (
        <p role="status" className="mb-4 text-sm text-emerald-700 dark:text-emerald-300">
          {notice}
        </p>
      ) : null}

      <div className="grid gap-6 lg:grid-cols-[3fr_2fr]">
        <section aria-labelledby="sections-heading">
          <h2 id="sections-heading" className="mb-1 text-sm font-bold text-stitch-fg">
            Sections
          </h2>
          <p className="mb-3 text-[11px] text-stitch-muted">
            {enabledCount} of {draft.sections.length} included. Tick a section to include it; reorder with the arrows
            or by dragging.
          </p>
          <SectionList definition={draft} specs={info.sections} onChange={setDraft} />
        </section>
        <section aria-labelledby="document-heading">
          <h2 id="document-heading" className="mb-3 text-sm font-bold text-stitch-fg">
            Document
          </h2>
          <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4">
            <DocumentSettingsForm
              value={draft.document}
              onChange={(document) => setDraft({ ...draft, document })}
            />
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <button type="button" className={btn} disabled={busy !== null || enabledCount === 0} onClick={preview}>
              {busy === 'preview' ? 'Preparing…' : 'Preview PDF'}
            </button>
            <button
              type="button"
              className={btnPrimary}
              disabled={busy !== null || enabledCount === 0}
              onClick={() => void generate('pdf')}
            >
              {busy === 'pdf' ? 'Generating…' : 'Download PDF'}
            </button>
            <button
              type="button"
              className={btnPrimary}
              disabled={busy !== null || enabledCount === 0}
              onClick={() => void generate('odt')}
            >
              {busy === 'odt' ? 'Generating…' : 'Download ODT'}
            </button>
          </div>
        </section>
      </div>

      <Dialog open={saveAsOpen} onClose={() => setSaveAsOpen(false)} title="Save report template">
        <form onSubmit={(e) => void saveAs(e)} className="space-y-3 text-sm">
          <label className="block">
            <span className="block mb-1">Name</span>
            <input
              className="w-full rounded-md border border-stitch-border bg-stitch-surface px-3 py-2"
              value={saveAsName}
              maxLength={120}
              onChange={(e) => setSaveAsName(e.target.value)}
              autoFocus
            />
          </label>
          <label className="inline-flex items-center gap-2">
            <input type="checkbox" checked={saveAsShared} onChange={(e) => setSaveAsShared(e.target.checked)} />
            Share with all project members
          </label>
          {error ? (
            <p role="alert" className="text-red-700 dark:text-red-300">
              {error}
            </p>
          ) : null}
          <div className="flex justify-end gap-2">
            <button type="button" className={btn} onClick={() => setSaveAsOpen(false)}>
              Cancel
            </button>
            <button type="submit" className={btnPrimary} disabled={saveAsName.trim() === '' || busy !== null}>
              {busy === 'save-as' ? 'Saving…' : 'Save'}
            </button>
          </div>
        </form>
      </Dialog>
    </div>
  );
}
