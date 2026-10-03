import { useState } from 'react';
import type {
  ReportDefinition,
  ReportOptionSpec,
  ReportSectionConfig,
  ReportSectionSpec,
} from '@/api/reports';
import { moveSection, setSectionOption, toggleSection } from '@/utils/reportDefinition';

type Props = {
  definition: ReportDefinition;
  specs: ReportSectionSpec[];
  onChange: (next: ReportDefinition) => void;
};

const iconBtn =
  'rounded-md p-1 text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-fg disabled:opacity-30 disabled:hover:bg-transparent';

function OptionControl({
  spec,
  section,
  onSet,
}: {
  spec: ReportOptionSpec;
  section: ReportSectionConfig;
  onSet: (value: unknown) => void;
}) {
  const value = section.options?.[spec.key] ?? spec.default;
  const label = <span className="block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1">{spec.label}</span>;
  if (spec.kind === 'select') {
    return (
      <label className="block">
        {label}
        <select
          className="rounded-md border border-stitch-border bg-stitch-surface px-2 py-1 text-xs"
          value={String(value)}
          onChange={(e) => onSet(e.target.value)}
        >
          {spec.choices.map((c) => (
            <option key={c.value} value={c.value}>
              {c.label}
            </option>
          ))}
        </select>
      </label>
    );
  }
  if (spec.kind === 'multi_select') {
    const selected = Array.isArray(value) ? (value as string[]) : [];
    return (
      <fieldset>
        <legend className="block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1">
          {spec.label}
        </legend>
        <div className="flex flex-wrap gap-x-4 gap-y-1">
          {spec.choices.map((c) => {
            const on = selected.includes(c.value);
            return (
              <label key={c.value} className="inline-flex items-center gap-1.5 text-xs">
                <input
                  type="checkbox"
                  checked={on}
                  // At least one value must stay selected.
                  disabled={on && selected.length === 1}
                  onChange={(e) =>
                    onSet(
                      spec.choices
                        .map((x) => x.value)
                        .filter((v) => (v === c.value ? e.target.checked : selected.includes(v))),
                    )
                  }
                />
                {c.label}
              </label>
            );
          })}
        </div>
      </fieldset>
    );
  }
  return (
    <label className="block">
      {label}
      <textarea
        className="w-full rounded-md border border-stitch-border bg-stitch-surface px-2 py-1.5 text-xs"
        rows={spec.multiline ? 4 : 1}
        value={String(value ?? '')}
        onChange={(e) => onSet(e.target.value)}
      />
    </label>
  );
}

/**
 * Report builder: sections in document order. Each can be turned on or off,
 * moved with the arrow buttons (keyboard) or by dragging, and configured.
 */
export default function SectionList({ definition, specs, onChange }: Props) {
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [dropAt, setDropAt] = useState<number | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const specByKey = new Map(specs.map((s) => [s.key, s]));
  const enabledOrder = definition.sections.filter((s) => s.enabled).map((s) => s.key);

  return (
    <ol className="space-y-2" aria-label="Report sections">
      {definition.sections.map((section, i) => {
        const spec = specByKey.get(section.key);
        const title = spec?.title ?? section.key;
        const position = enabledOrder.indexOf(section.key);
        return (
          <li
            key={section.key}
            draggable
            onDragStart={(e) => {
              setDragFrom(i);
              e.dataTransfer.effectAllowed = 'move';
            }}
            onDragOver={(e) => {
              if (dragFrom === null) return;
              e.preventDefault();
              setDropAt(i);
            }}
            onDragLeave={() => setDropAt((d) => (d === i ? null : d))}
            onDrop={(e) => {
              e.preventDefault();
              if (dragFrom !== null) onChange(moveSection(definition, dragFrom, i));
              setDragFrom(null);
              setDropAt(null);
            }}
            onDragEnd={() => {
              setDragFrom(null);
              setDropAt(null);
            }}
            className={`rounded-lg border bg-stitch-surface ${
              dropAt === i && dragFrom !== i ? 'border-stitch-accent' : 'border-stitch-border'
            } ${section.enabled ? '' : 'opacity-60'}`}
          >
            <div className="flex items-start gap-2 px-3 py-2">
              <span
                className="material-symbols-outlined text-stitch-muted cursor-grab select-none text-lg mt-0.5"
                aria-hidden
                title="Drag to reorder"
              >
                drag_indicator
              </span>
              <input
                type="checkbox"
                className="mt-1"
                checked={section.enabled}
                aria-label={`Include ${title}`}
                onChange={(e) => onChange(toggleSection(definition, section.key, e.target.checked))}
              />
              <div className="min-w-0 flex-1">
                <p className="text-sm font-semibold text-stitch-fg">
                  {position >= 0 ? <span className="text-stitch-muted mr-1.5">{position + 1}.</span> : null}
                  {title}
                </p>
                {spec?.description ? (
                  <p className="text-[11px] text-stitch-muted">{spec.description}</p>
                ) : null}
              </div>
              {spec && spec.options.length > 0 ? (
                <button
                  type="button"
                  className={iconBtn}
                  aria-expanded={open === section.key}
                  aria-label={`Options of ${title}`}
                  onClick={() => setOpen((o) => (o === section.key ? null : section.key))}
                >
                  <span className="material-symbols-outlined text-lg">tune</span>
                </button>
              ) : null}
              <button
                type="button"
                className={iconBtn}
                aria-label={`Move ${title} up`}
                disabled={i === 0}
                onClick={() => onChange(moveSection(definition, i, i - 1))}
              >
                <span className="material-symbols-outlined text-lg">arrow_upward</span>
              </button>
              <button
                type="button"
                className={iconBtn}
                aria-label={`Move ${title} down`}
                disabled={i === definition.sections.length - 1}
                onClick={() => onChange(moveSection(definition, i, i + 1))}
              >
                <span className="material-symbols-outlined text-lg">arrow_downward</span>
              </button>
            </div>
            {spec && open === section.key ? (
              <div className="space-y-3 border-t border-stitch-border px-10 py-3">
                {spec.options.map((option) => (
                  <OptionControl
                    key={option.key}
                    spec={option}
                    section={section}
                    onSet={(value) => onChange(setSectionOption(definition, section.key, option.key, value))}
                  />
                ))}
              </div>
            ) : null}
          </li>
        );
      })}
    </ol>
  );
}
