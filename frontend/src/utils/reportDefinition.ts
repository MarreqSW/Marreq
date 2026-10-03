import type { ReportDefinition, ReportSectionConfig } from '@/api/reports';

/** Deep copy, so edits never touch a definition held elsewhere. */
export function cloneDefinition(def: ReportDefinition): ReportDefinition {
  return JSON.parse(JSON.stringify(def)) as ReportDefinition;
}

function withSections(def: ReportDefinition, sections: ReportSectionConfig[]): ReportDefinition {
  return { ...def, sections };
}

/** Move the section at `from` to position `to` (both clamped). */
export function moveSection(def: ReportDefinition, from: number, to: number): ReportDefinition {
  const sections = [...def.sections];
  if (from < 0 || from >= sections.length) return def;
  const target = Math.max(0, Math.min(sections.length - 1, to));
  if (target === from) return def;
  const [item] = sections.splice(from, 1);
  sections.splice(target, 0, item);
  return withSections(def, sections);
}

export function toggleSection(def: ReportDefinition, key: string, enabled: boolean): ReportDefinition {
  return withSections(
    def,
    def.sections.map((s) => (s.key === key ? { ...s, enabled } : s)),
  );
}

/** Set one option; `undefined` removes it so the server default applies. */
export function setSectionOption(
  def: ReportDefinition,
  key: string,
  option: string,
  value: unknown,
): ReportDefinition {
  return withSections(
    def,
    def.sections.map((s) => {
      if (s.key !== key) return s;
      const options = { ...(s.options ?? {}) };
      if (value === undefined) delete options[option];
      else options[option] = value;
      return { ...s, options };
    }),
  );
}

/** Stable comparison for "unsaved changes". */
export function sameDefinition(a: ReportDefinition | null, b: ReportDefinition | null): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
