import { afterEach, describe, vi } from 'vitest';
import { CSRF, itSendsEachRequest } from '@/test/apiHarness';
import {
  commitExcelImport,
  commitReqifImport,
  importProjectBundle,
  previewExcelImport,
} from '../imports';

const xlsx = new File(['x'], 'reqs.xlsx');
const reqifz = new File(['x'], 'spec.reqifz');
const bundle = new File(['{}'], 'bundle.json');

describe('imports API (multipart)', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'previewExcelImport',
      call: () => previewExcelImport(5, xlsx, CSRF),
      method: 'POST',
      url: '/api/projects/5/imports/excel/preview',
      body: { file: 'file:reqs.xlsx' },
    },
    {
      name: 'commitReqifImport',
      call: () => commitReqifImport(5, reqifz, CSRF),
      method: 'POST',
      url: '/api/projects/5/imports/reqif',
      body: { file: 'file:spec.reqifz' },
    },
    {
      name: 'commitExcelImport sends the mappings as JSON fields',
      call: () =>
        commitExcelImport(
          5,
          xlsx,
          'matrix',
          [{ excel_column: 'Code', target_field: 'requirement_reference_code' }],
          [{ target_field: 'status', source_value: 'Open', target_id: 3 }],
          CSRF,
        ),
      method: 'POST',
      url: '/api/projects/5/imports/excel',
      body: {
        file: 'file:reqs.xlsx',
        import_type: 'matrix',
        column_mappings: '[{"excel_column":"Code","target_field":"requirement_reference_code"}]',
        value_mappings: '[{"target_field":"status","source_value":"Open","target_id":3}]',
      },
    },
    {
      name: 'importProjectBundle into the personal namespace',
      call: () => importProjectBundle(bundle, CSRF),
      method: 'POST',
      url: '/api/projects/imports/bundle',
      body: { file: 'file:bundle.json' },
    },
    {
      name: 'importProjectBundle into a group',
      call: () => importProjectBundle(bundle, CSRF, 4),
      method: 'POST',
      url: '/api/projects/imports/bundle',
      body: { file: 'file:bundle.json', group_id: '4' },
    },
  ]);
});
