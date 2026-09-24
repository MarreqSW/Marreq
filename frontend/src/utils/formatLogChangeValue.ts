/** Resolves catalog foreign keys in audit change rows to display titles. */

export type CatalogLabelMaps = {
  requirementStatusById: Map<number, string>;
  verificationStatusById: Map<number, string>;
  categoryById: Map<number, string>;
  applicabilityById: Map<number, string>;
  methodById: Map<number, string>;
};

const ID_FIELDS = new Set([
  'Status',
  'Category',
  'Applicability',
  'Verification type',
  'Verification',
]);

function parseIds(raw: string): number[] | null {
  const t = raw.trim();
  if (t === '' || t === '—') return null;
  const asNum = Number(t);
  if (Number.isFinite(asNum) && !t.startsWith('[')) return [asNum];
  if (t.startsWith('[')) {
    try {
      const parsed = JSON.parse(t) as unknown;
      if (Array.isArray(parsed) && parsed.every((n) => typeof n === 'number')) {
        return parsed;
      }
    } catch {
      return null;
    }
  }
  return null;
}

function lookup(map: Map<number, string>, id: number): string {
  if (id === 0) return '—';
  return map.get(id) ?? String(id);
}

function isVerificationEntity(entityType: string): boolean {
  const u = entityType.toUpperCase();
  return u === 'VERIFICATION' || u === 'TEST' || u === 'TESTS';
}

function mapForField(
  field: string,
  entityType: string,
  maps: CatalogLabelMaps,
): Map<number, string> | null {
  switch (field) {
    case 'Status':
      return isVerificationEntity(entityType)
        ? maps.verificationStatusById
        : maps.requirementStatusById;
    case 'Category':
      return maps.categoryById;
    case 'Applicability':
      return maps.applicabilityById;
    case 'Verification type':
    case 'Verification':
      return maps.methodById;
    default:
      return null;
  }
}

/** Turns stored catalog ids into titles; leaves other values unchanged. */
export function formatLogChangeValue(
  field: string,
  raw: string,
  entityType: string,
  maps: CatalogLabelMaps,
): string {
  if (!ID_FIELDS.has(field)) return raw;
  const ids = parseIds(raw);
  if (!ids) return raw;
  const map = mapForField(field, entityType, maps);
  if (!map) return raw;
  return ids.map((id) => lookup(map, id)).join(', ');
}
