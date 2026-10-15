import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import UsedInUseCasesCard from '@/components/useCases/UsedInUseCasesCard';

describe('UsedInUseCasesCard', () => {
  it('lists the use cases and steps that link here, marking suspect links', async () => {
    const load = vi.fn().mockResolvedValue([
      { use_case_id: 1, reference_code: 'UC-001', title: 'Export requirements', link_id: 5, element_id: 's_C3mz', element_label: '3', suspect: false },
      { use_case_id: 4, reference_code: 'UC-004', title: 'Downlink telemetry pass', link_id: 9, element_id: null, element_label: null, suspect: true },
    ]);
    render(
      <MemoryRouter>
        <UsedInUseCasesCard basePath="/space-project" load={load} />
      </MemoryRouter>,
    );
    const first = await screen.findByRole('link', { name: /UC-001/ });
    expect(first).toHaveAttribute('href', '/space-project/use-cases/1');
    expect(first).toHaveTextContent('step 3');
    expect(screen.getByRole('link', { name: /UC-004/ })).toContainElement(screen.getByLabelText('Suspect link'));
  });

  it('shows None when the loader fails or finds nothing', async () => {
    render(
      <MemoryRouter>
        <UsedInUseCasesCard basePath="/p" load={() => Promise.reject(new Error('403'))} variant="section" />
      </MemoryRouter>,
    );
    expect(await screen.findByText('None')).toBeInTheDocument();
  });
});
