import { afterEach, describe, vi } from 'vitest';
import { CSRF, itSendsEachRequest } from '@/test/apiHarness';
import { createVerificationStatus, updateVerificationStatus } from '../catalog';
import {
  getProjectCloseOut,
  getRequirementCloseOut,
  getVerificationControl,
  putRequirementCompliance,
  putVerificationControl,
} from '../verificationControl';

const status = { title: 'Waived', description: '', tag: 'W', project_id: 5 };

describe('verification control API (issue #353)', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'getVerificationControl',
      call: () => getVerificationControl(5, 20),
      method: 'GET',
      url: '/api/projects/5/verifications/20/control',
    },
    {
      name: 'putVerificationControl',
      call: () =>
        putVerificationControl(
          5,
          20,
          { verification_level: 'Subsystem', verification_stage: 'QUAL', evidence_reference: '' },
          CSRF,
        ),
      method: 'PUT',
      url: '/api/projects/5/verifications/20/control',
      body: { verification_level: 'Subsystem', verification_stage: 'QUAL', evidence_reference: '' },
    },
    {
      name: 'getRequirementCloseOut',
      call: () => getRequirementCloseOut(5, 1),
      method: 'GET',
      url: '/api/projects/5/requirements/1/close_out',
    },
    {
      name: 'getProjectCloseOut',
      call: () => getProjectCloseOut(5),
      method: 'GET',
      url: '/api/projects/5/close_out',
    },
    {
      name: 'putRequirementCompliance sets an assessment',
      call: () => putRequirementCompliance(5, 1, 'PC', 'RFW-3', CSRF),
      method: 'PUT',
      url: '/api/projects/5/requirements/1/compliance',
      body: { compliance: 'PC', note: 'RFW-3' },
    },
    {
      name: 'putRequirementCompliance clears it with null',
      call: () => putRequirementCompliance(5, 1, null, null, CSRF),
      method: 'PUT',
      url: '/api/projects/5/requirements/1/compliance',
      body: { compliance: null, note: null },
    },
    {
      name: 'createVerificationStatus sends an explicit outcome',
      call: () => createVerificationStatus({ ...status, outcome: 'passed' }, CSRF),
      method: 'POST',
      url: '/api/verification-status',
      body: { ...status, tag_color: null, outcome: 'passed' },
      response: { json: { id: 3 } },
    },
    {
      name: 'updateVerificationStatus sends the outcome',
      call: () => updateVerificationStatus(3, { ...status, outcome: 'failed' }, CSRF),
      method: 'PUT',
      url: '/api/verification-status/3',
      body: { id: 3, ...status, is_system: false, tag_color: null, outcome: 'failed' },
    },
  ]);
});
