import type { ReportDocumentSettings } from '@/api/reports';

type Props = {
  value: ReportDocumentSettings;
  onChange: (next: ReportDocumentSettings) => void;
};

const labelCls = 'block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1';
const inputCls =
  'w-full rounded-md border border-stitch-border bg-stitch-surface px-2 py-1.5 text-xs text-stitch-fg';

type RowList<K extends 'signatories' | 'change_record' | 'documents'> = {
  field: K;
  title: string;
  columns: { key: keyof ReportDocumentSettings[K][number] & string; label: string; wide?: boolean }[];
  empty: ReportDocumentSettings[K][number];
  addLabel: string;
};

function Rows<K extends 'signatories' | 'change_record' | 'documents'>({
  list,
  value,
  onChange,
}: {
  list: RowList<K>;
  value: ReportDocumentSettings;
  onChange: (next: ReportDocumentSettings) => void;
}) {
  const rows = value[list.field] as Record<string, string>[];
  const set = (next: Record<string, string>[]) =>
    onChange({ ...value, [list.field]: next } as ReportDocumentSettings);
  return (
    <fieldset className="space-y-1.5">
      <legend className={labelCls}>{list.title}</legend>
      {rows.map((row, i) => (
        <div key={i} className="flex gap-1.5">
          {list.columns.map((c) => (
            <input
              key={c.key}
              className={`${inputCls} ${c.wide ? 'flex-[2]' : 'flex-1'}`}
              aria-label={`${list.title} ${i + 1} ${c.label}`}
              placeholder={c.label}
              value={row[c.key] ?? ''}
              onChange={(e) => set(rows.map((r, j) => (j === i ? { ...r, [c.key]: e.target.value } : r)))}
            />
          ))}
          <button
            type="button"
            className="rounded-md px-1.5 text-stitch-muted hover:text-red-600"
            aria-label={`Remove ${list.title} ${i + 1}`}
            onClick={() => set(rows.filter((_, j) => j !== i))}
          >
            <span className="material-symbols-outlined text-base">close</span>
          </button>
        </div>
      ))}
      <button
        type="button"
        className="text-[11px] font-semibold text-stitch-accent hover:underline"
        onClick={() => set([...rows, { ...(list.empty as Record<string, string>) }])}
      >
        {list.addLabel}
      </button>
    </fieldset>
  );
}

/** Report builder: fields printed on the cover, header and front matter. */
export default function DocumentSettingsForm({ value, onChange }: Props) {
  const set = (patch: Partial<ReportDocumentSettings>) => onChange({ ...value, ...patch });
  const text = (key: 'doc_id' | 'title' | 'issue' | 'revision' | 'classification', label: string, placeholder = '') => (
    <label className="block">
      <span className={labelCls}>{label}</span>
      <input className={inputCls} value={value[key]} placeholder={placeholder} onChange={(e) => set({ [key]: e.target.value })} />
    </label>
  );
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-3">
        <div className="col-span-2">{text('doc_id', 'Document ID')}</div>
        <div className="col-span-2">{text('title', 'Title', 'Report type title')}</div>
        {text('issue', 'Issue')}
        {text('revision', 'Revision')}
        <div className="col-span-2">{text('classification', 'Classification', 'e.g. Company internal')}</div>
        <label className="block">
          <span className={labelCls}>Watermark (PDF)</span>
          <input
            className={inputCls}
            value={value.watermark ?? ''}
            placeholder="e.g. DRAFT"
            onChange={(e) => set({ watermark: e.target.value === '' ? null : e.target.value })}
          />
        </label>
        <label className="block">
          <span className={labelCls}>Page size</span>
          <select
            className={inputCls}
            value={value.page_size}
            onChange={(e) => set({ page_size: e.target.value as ReportDocumentSettings['page_size'] })}
          >
            <option value="a4">A4</option>
            <option value="letter">US Letter</option>
          </select>
        </label>
        <label className="col-span-2 inline-flex items-center gap-2 text-xs text-stitch-fg">
          <input type="checkbox" checked={value.pdf_a} onChange={(e) => set({ pdf_a: e.target.checked })} />
          Archival PDF/A-2b
        </label>
      </div>
      <Rows
        list={{
          field: 'signatories',
          title: 'Signatories',
          columns: [
            { key: 'role', label: 'Role' },
            { key: 'name', label: 'Name', wide: true },
          ],
          empty: { role: '', name: '' },
          addLabel: 'Add signatory',
        }}
        value={value}
        onChange={onChange}
      />
      <Rows
        list={{
          field: 'change_record',
          title: 'Change record',
          columns: [
            { key: 'issue', label: 'Issue' },
            { key: 'date', label: 'Date' },
            { key: 'changes', label: 'Changes', wide: true },
            { key: 'author', label: 'Author' },
          ],
          empty: { issue: '', date: '', changes: '', author: '' },
          addLabel: 'Add issue',
        }}
        value={value}
        onChange={onChange}
      />
      <Rows
        list={{
          field: 'documents',
          title: 'Reference documents',
          columns: [
            { key: 'ref', label: 'Ref.' },
            { key: 'id', label: 'Document' },
            { key: 'title', label: 'Title', wide: true },
            { key: 'issue', label: 'Issue' },
          ],
          empty: { ref: '', id: '', title: '', issue: '' },
          addLabel: 'Add document',
        }}
        value={value}
        onChange={onChange}
      />
    </div>
  );
}
