import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { downloadReport, listReportTemplates, listReportTypes } from '@/api/client';
import type { ReportFormat, ReportTemplate, ReportTypeInfo, ReportTypeKey } from '@/api/reports';
import { ReportSection } from './ReportSection';

type Props = {
  projectId: number;
  basePath: string;
  csrfToken: string;
};

const selectCls = 'rounded-md border border-stitch-border bg-stitch-surface px-2 py-1.5 text-sm';
const labelCls = 'block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1';

/** Reports page: generate a report document from the default or a saved template (issue #355). */
export default function ReportDocumentsCard({ projectId, basePath, csrfToken }: Props) {
  const [types, setTypes] = useState<ReportTypeInfo[] | null>(null);
  const [templates, setTemplates] = useState<ReportTemplate[]>([]);
  const [type, setType] = useState<ReportTypeKey>('vcd');
  const [templateId, setTemplateId] = useState<number | null>(null);
  const [format, setFormat] = useState<ReportFormat>('pdf');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([listReportTypes(projectId), listReportTemplates(projectId)])
      .then(([t, tpl]) => {
        if (cancelled) return;
        setTypes(t);
        setTemplates(tpl);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(e instanceof Error ? e.message : 'Could not load report types.');
      });
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const forType = templates.filter((t) => t.report_type === type);
  const info = types?.find((t) => t.key === type);
  const customize = `${basePath}/reports/builder?${new URLSearchParams({
    type,
    ...(templateId != null ? { template: String(templateId) } : {}),
  })}`;

  async function generate() {
    setBusy(true);
    setError(null);
    try {
      await downloadReport(
        projectId,
        type,
        format,
        templateId != null ? { template_id: templateId } : {},
        csrfToken,
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Could not generate the report.');
    } finally {
      setBusy(false);
    }
  }

  return (
    <ReportSection
      title="Report documents"
      subtitle="Verification Control Document and traceability & coverage report, as PDF or ODT"
    >
      <div className="p-4 space-y-3">
        {types ? (
          <>
            <div className="flex flex-wrap items-end gap-3">
              <label className="block">
                <span className={labelCls}>Report</span>
                <select
                  className={selectCls}
                  value={type}
                  onChange={(e) => {
                    setType(e.target.value as ReportTypeKey);
                    setTemplateId(null);
                  }}
                >
                  {types.map((t) => (
                    <option key={t.key} value={t.key}>
                      {t.title}
                    </option>
                  ))}
                </select>
              </label>
              <label className="block">
                <span className={labelCls}>Template</span>
                <select
                  className={selectCls}
                  value={templateId ?? ''}
                  onChange={(e) => setTemplateId(e.target.value === '' ? null : Number(e.target.value))}
                >
                  <option value="">Default</option>
                  {forType.map((t) => (
                    <option key={t.id} value={t.id}>
                      {t.name}
                      {t.visibility === 'shared' ? ' (shared)' : ''}
                    </option>
                  ))}
                </select>
              </label>
              <fieldset className="flex items-center gap-3 text-sm">
                <legend className={labelCls}>Format</legend>
                {(['pdf', 'odt'] as const).map((f) => (
                  <label key={f} className="inline-flex items-center gap-1.5">
                    <input type="radio" name="report-format" checked={format === f} onChange={() => setFormat(f)} />
                    {f.toUpperCase()}
                  </label>
                ))}
              </fieldset>
              <button
                type="button"
                className="rounded-md bg-stitch-accent px-4 py-1.5 text-sm font-semibold text-white disabled:opacity-50"
                disabled={busy}
                onClick={() => void generate()}
              >
                {busy ? 'Generating…' : 'Generate'}
              </button>
              <Link to={customize} className="text-sm font-semibold text-stitch-accent hover:underline">
                Customize…
              </Link>
            </div>
            {info ? <p className="text-[11px] text-stitch-muted">{info.description}</p> : null}
          </>
        ) : error ? null : (
          <p className="text-xs text-stitch-muted">Loading…</p>
        )}
        {error ? (
          <p role="alert" className="text-xs text-red-700 dark:text-red-300">
            {error}
          </p>
        ) : null}
      </div>
    </ReportSection>
  );
}
