import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import BaselineUseCasesSection from '@/components/useCases/BaselineUseCasesSection';

vi.mock('@/api/client', () => ({ listBaselineUseCases: vi.fn() }));

describe('BaselineUseCasesSection', () => {
  it('lists recorded versions and links changed ones to the comparison', async () => {
    vi.mocked(apiClient.listBaselineUseCases).mockResolvedValue([
      { use_case_id: 1, reference_code: 'UC-001', title: 'Export', primary_actor: null, version_id: 31, version_number: 1, approval_state: 'approved', current_version_id: 32, current_version_number: 2, link_count: 3 },
      { use_case_id: 4, reference_code: 'UC-004', title: 'Downlink', primary_actor: null, version_id: 40, version_number: 1, approval_state: 'draft', current_version_id: 40, current_version_number: 1, link_count: 0 },
    ]);
    render(
      <MemoryRouter>
        <BaselineUseCasesSection projectId={5} baselineId={9} basePath="/space-project" />
      </MemoryRouter>,
    );
    expect(await screen.findByRole('link', { name: 'UC-001' })).toHaveAttribute('href', '/space-project/use-cases/1?version=31');
    expect(screen.getByRole('link', { name: 'v2: compare' })).toHaveAttribute(
      'href',
      '/space-project/use-cases/1?view=review&from=31',
    );
    expect(screen.getByText('unchanged')).toBeInTheDocument();
    expect(screen.getByText('Use cases (2)')).toBeInTheDocument();
  });
});
