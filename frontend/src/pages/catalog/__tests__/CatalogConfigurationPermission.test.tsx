import { render, screen } from '@testing-library/react';
import type { ComponentType } from 'react';
import { MemoryRouter, Outlet, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { EffectivePermissions } from '@/api/types';
import CatalogApplicabilityPage from '../CatalogApplicabilityPage';
import CatalogCategoriesPage from '../CatalogCategoriesPage';
import CatalogRequirementStatusesPage from '../CatalogRequirementStatusesPage';
import CatalogVerificationMethodsPage from '../CatalogVerificationMethodsPage';
import CatalogVerificationStatusesPage from '../CatalogVerificationStatusesPage';

vi.mock('@/api/client');
vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf' }),
}));

function renderPage(Page: ComponentType) {
  return render(
    <MemoryRouter initialEntries={['/p/catalog']}>
      <Routes>
        <Route
          path="/p"
          element={<Outlet context={{ projectId: 5, basePath: '/p', globalSearch: '', setGlobalSearch: vi.fn() }} />}
        >
          <Route path="catalog" element={<Page />} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

const PAGES: [string, ComponentType][] = [
  ['categories', CatalogCategoriesPage],
  ['applicability', CatalogApplicabilityPage],
  ['requirement statuses', CatalogRequirementStatusesPage],
  ['verification statuses', CatalogVerificationStatusesPage],
  ['verification methods', CatalogVerificationMethodsPage],
];

// Issue #288: the backend requires "manage project configuration" (project
// admin) for every catalog change; edit rights alone must not enable the forms.
describe('catalog pages need manage project configuration', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.listCategories).mockResolvedValue([]);
    vi.mocked(apiClient.listApplicability).mockResolvedValue([]);
    vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([]);
  });

  it.each(PAGES)('%s: read-only for a user who can only edit requirements', async (_name, Page) => {
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      edit_requirements: true,
      manage_project_configuration: false,
    } as EffectivePermissions);
    renderPage(Page);
    expect(await screen.findByText('manage project configuration')).toBeInTheDocument();
    for (const button of screen.getAllByRole('button')) expect(button).toBeDisabled();
  });

  it.each(PAGES)('%s: editable for a project admin', async (_name, Page) => {
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      manage_project_configuration: true,
    } as EffectivePermissions);
    renderPage(Page);
    await vi.waitFor(() => expect(apiClient.getMyPermissions).toHaveBeenCalled());
    await vi.waitFor(() =>
      expect(screen.getAllByRole('button').some((button) => !(button as HTMLButtonElement).disabled)).toBe(true),
    );
    expect(screen.queryByText('manage project configuration')).not.toBeInTheDocument();
  });
});
