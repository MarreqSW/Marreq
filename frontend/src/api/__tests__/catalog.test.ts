import { afterEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, stubFetch } from '@/test/apiHarness';
import {
  createApplicability,
  createCategory,
  createCustomField,
  createRequirementStatus,
  createVerificationMethod,
  createVerificationStatus,
  deleteApplicability,
  deleteCategory,
  deleteCustomField,
  deleteRequirementStatus,
  deleteVerificationMethod,
  deleteVerificationStatus,
  listApplicability,
  listCategories,
  listCustomFieldsByProject,
  listRequirementStatuses,
  listVerificationMethods,
  listVerificationStatuses,
  updateApplicability,
  updateCategory,
  updateCustomField,
  updateRequirementStatus,
  updateVerificationMethod,
  updateVerificationStatus,
} from '../catalog';

const tagged = { title: 'Safety', description: 'Safety related', tag: 'SAF', project_id: 5 };
const status = { ...tagged, tag_color: '#ff0000' };
const field = { label: 'Owner', field_type: 'text' } as Parameters<typeof createCustomField>[1];

describe('catalog API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    { name: 'listCategories', call: () => listCategories(), method: 'GET', url: '/api/categories' },
    {
      name: 'createCategory sends a null id and returns the new id',
      call: () => createCategory(tagged, CSRF),
      method: 'POST',
      url: '/api/categories',
      body: { ...tagged, id: null },
      response: { json: { id: 9, status: 'ok' } },
      returns: { id: 9 },
    },
    {
      name: 'updateCategory puts the id in the body',
      call: () => updateCategory(3, tagged, CSRF),
      method: 'PUT',
      url: '/api/categories/3',
      body: { ...tagged, id: 3 },
    },
    { name: 'deleteCategory', call: () => deleteCategory(3, CSRF), method: 'DELETE', url: '/api/categories/3' },

    { name: 'listApplicability', call: () => listApplicability(), method: 'GET', url: '/api/applicability' },
    {
      name: 'createApplicability',
      call: () => createApplicability({ ...tagged, id: 4 }, CSRF),
      method: 'POST',
      url: '/api/applicability',
      body: { ...tagged, id: 4 },
      response: { json: { id: 4 } },
      returns: { id: 4 },
    },
    {
      name: 'updateApplicability',
      call: () => updateApplicability(4, tagged, CSRF),
      method: 'PUT',
      url: '/api/applicability/4',
      body: { ...tagged, id: 4 },
    },
    { name: 'deleteApplicability', call: () => deleteApplicability(4, CSRF), method: 'DELETE', url: '/api/applicability/4' },

    { name: 'listRequirementStatuses', call: () => listRequirementStatuses(), method: 'GET', url: '/api/status' },
    {
      name: 'createRequirementStatus sends only the writable fields',
      call: () => createRequirementStatus({ ...status, is_system: true } as never, CSRF),
      method: 'POST',
      url: '/api/status',
      body: status,
      response: { json: { id: 11, extra: true } },
      returns: { id: 11 },
    },
    {
      name: 'updateRequirementStatus defaults is_system and tag_color',
      call: () => updateRequirementStatus(11, tagged as never, CSRF),
      method: 'PUT',
      url: '/api/status/11',
      body: { id: 11, ...tagged, is_system: false, tag_color: null },
    },
    {
      name: 'deleteRequirementStatus',
      call: () => deleteRequirementStatus(11, CSRF),
      method: 'DELETE',
      url: '/api/status/11',
    },

    {
      name: 'listVerificationStatuses',
      call: () => listVerificationStatuses(),
      method: 'GET',
      url: '/api/verification-status',
    },
    {
      name: 'createVerificationStatus',
      call: () => createVerificationStatus(tagged as never, CSRF),
      method: 'POST',
      url: '/api/verification-status',
      body: { ...tagged, tag_color: null },
      response: { json: { id: 12 } },
      returns: { id: 12 },
    },
    {
      name: 'updateVerificationStatus keeps an explicit is_system',
      call: () => updateVerificationStatus(12, { ...status, is_system: true } as never, CSRF),
      method: 'PUT',
      url: '/api/verification-status/12',
      body: { id: 12, ...status, is_system: true },
    },
    {
      name: 'deleteVerificationStatus',
      call: () => deleteVerificationStatus(12, CSRF),
      method: 'DELETE',
      url: '/api/verification-status/12',
    },

    {
      name: 'listCustomFieldsByProject',
      call: () => listCustomFieldsByProject(5),
      method: 'GET',
      url: '/api/projects/5/custom_fields',
    },
    {
      name: 'createCustomField',
      call: () => createCustomField(5, field, CSRF),
      method: 'POST',
      url: '/api/projects/5/custom_fields',
      body: field,
      response: { json: { id: 21 } },
      returns: { id: 21 },
    },
    {
      name: 'updateCustomField',
      call: () => updateCustomField(5, 21, field, CSRF),
      method: 'PUT',
      url: '/api/projects/5/custom_fields/21',
      body: field,
    },
    {
      name: 'deleteCustomField',
      call: () => deleteCustomField(5, 21, CSRF),
      method: 'DELETE',
      url: '/api/projects/5/custom_fields/21',
    },

    {
      name: 'listVerificationMethods',
      call: () => listVerificationMethods(),
      method: 'GET',
      url: '/api/verification-methods',
    },
    {
      name: 'createVerificationMethod scopes the body to the project',
      call: () => createVerificationMethod(5, { title: 'Test', description: '', tag: 'T' } as never, CSRF),
      method: 'POST',
      url: '/api/projects/5/verification-methods',
      body: { title: 'Test', description: '', tag: 'T', id: null, project_id: 5 },
      response: { json: { id: 31 } },
      returns: { id: 31 },
    },
    {
      name: 'updateVerificationMethod',
      call: () => updateVerificationMethod(5, 31, { title: 'Test', description: '', tag: 'T' } as never, CSRF),
      method: 'PUT',
      url: '/api/projects/5/verification-methods/31',
      body: { title: 'Test', description: '', tag: 'T', id: 31, project_id: 5 },
    },
    {
      name: 'deleteVerificationMethod',
      call: () => deleteVerificationMethod(5, 31, CSRF),
      method: 'DELETE',
      url: '/api/projects/5/verification-methods/31',
    },
  ]);

  it.each([
    ['createApplicability', () => createApplicability(tagged, CSRF)],
    ['createRequirementStatus', () => createRequirementStatus(status as never, CSRF)],
    ['createVerificationStatus', () => createVerificationStatus(status as never, CSRF)],
  ])('%s throws the server message, or the status text when the body is empty', async (_n, call) => {
    stubFetch({ status: 409, text: 'tag already exists' });
    await expect(call()).rejects.toThrow('tag already exists');
    stubFetch({ status: 500, text: '' });
    await expect(call()).rejects.toThrow('Error');
  });

  it('fetchJson-based calls surface the API error message', async () => {
    stubFetch({ status: 403, json: { message: 'Access denied.' } });
    await expect(deleteCategory(1, CSRF)).rejects.toMatchObject({ status: 403, message: 'Access denied.' });
  });
});
