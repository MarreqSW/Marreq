import { afterEach, describe, vi } from 'vitest';
import { CSRF, itSendsEachRequest } from '@/test/apiHarness';
import {
  createVerification,
  deleteVerificationGlobally,
  getVerification,
  getVerificationMatrix,
  listMatrix,
  listVerificationActivityByProject,
  listVerificationMethodsByProject,
  listVerifications,
  listVerificationsByProject,
  listVerificationSnapshotsByProject,
  putVerificationMatrix,
  updateVerificationField,
} from '../verifications';

describe('verifications API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    { name: 'listVerifications', call: () => listVerifications(), method: 'GET', url: '/api/verifications' },
    {
      name: 'listVerificationsByProject',
      call: () => listVerificationsByProject(5),
      method: 'GET',
      url: '/api/projects/5/verifications',
    },
    {
      name: 'listVerificationMethodsByProject',
      call: () => listVerificationMethodsByProject(5),
      method: 'GET',
      url: '/api/projects/5/verification-methods',
    },
    {
      name: 'createVerification returns the new id',
      call: () => createVerification({ name: 'Thermal vacuum', project_id: 5 } as never, CSRF),
      method: 'POST',
      url: '/api/verifications',
      body: { name: 'Thermal vacuum', project_id: 5 },
      response: { json: { status: 'created', id: 77 } },
      returns: { id: 77 },
    },
    { name: 'getVerification', call: () => getVerification(77), method: 'GET', url: '/api/verifications/77' },
    {
      name: 'updateVerificationField',
      call: () => updateVerificationField(5, 77, 'status_id', '3', CSRF),
      method: 'POST',
      url: '/api/projects/5/verifications/77/field',
      body: { field: 'status_id', value: '3' },
    },
    {
      name: 'deleteVerificationGlobally',
      call: () => deleteVerificationGlobally(77, CSRF),
      method: 'DELETE',
      url: '/api/verifications/77',
    },
    {
      name: 'listVerificationActivityByProject',
      call: () => listVerificationActivityByProject(5, 77),
      method: 'GET',
      url: '/api/projects/5/verifications/77/activity',
    },
    {
      name: 'listVerificationSnapshotsByProject',
      call: () => listVerificationSnapshotsByProject(5, 77),
      method: 'GET',
      url: '/api/projects/5/verifications/77/snapshots',
    },
    { name: 'listMatrix', call: () => listMatrix(5), method: 'GET', url: '/api/projects/5/matrix' },
    {
      name: 'getVerificationMatrix',
      call: () => getVerificationMatrix(5, 77),
      method: 'GET',
      url: '/api/projects/5/verifications/77/matrix',
    },
    {
      name: 'putVerificationMatrix replaces the links',
      call: () => putVerificationMatrix(5, 77, { requirement_ids: [1, 2] } as never, CSRF),
      method: 'PUT',
      url: '/api/projects/5/verifications/77/matrix',
      body: { requirement_ids: [1, 2] },
    },
  ]);
});
