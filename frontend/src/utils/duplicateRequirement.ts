import type { Requirement } from '@/api/types';

export function duplicateRequirementTitle(title: string): string {
  const trimmed = title.trim();
  return `${trimmed || 'Untitled requirement'} (Copy)`;
}

export function nextDuplicateReference(
  sourceReference: string,
  requirements: Pick<Requirement, 'reference_code'>[],
): string {
  const source = sourceReference.trim();
  const used = new Set(
    requirements
      .map((requirement) => requirement.reference_code.trim().toLowerCase())
      .filter(Boolean),
  );

  const numericSuffix = source.match(/^(.*?)(\d+)$/);
  if (numericSuffix) {
    const prefix = numericSuffix[1];
    const digits = numericSuffix[2];
    let value = Number(digits) + 1;
    while (true) {
      const candidate = `${prefix}${String(value).padStart(digits.length, '0')}`;
      if (!used.has(candidate.toLowerCase())) return candidate;
      value += 1;
    }
  }

  const base = source ? `${source}-COPY` : 'REQ-COPY';
  if (!used.has(base.toLowerCase())) return base;
  let copy = 2;
  while (used.has(`${base}-${copy}`.toLowerCase())) copy += 1;
  return `${base}-${copy}`;
}
