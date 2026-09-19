import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useOutletContext } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import ImportPage from '../ImportPage';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import * as apiClient from '@/api/client';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({
    csrfToken: 'csrf-test',
    dashboard: {
      projects: [{ id: 5, name: 'Space Project' }],
    },
  }),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    ...actual,
    useOutletContext: vi.fn(),
  };
});

describe('ImportPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(useOutletContext).mockReturnValue({
      projectId: 5,
      basePath: '/space-project',
      globalSearch: '',
      setGlobalSearch: vi.fn(),
    } satisfies ProjectOutletContext);
    vi.mocked(apiClient.getMyPermissions).mockResolvedValue({
      view_requirements: true,
      edit_requirements: true,
      approve_versions: true,
      is_project_reviewer: true,
      manage_custom_fields: true,
      manage_project_members: true,
    });
    vi.mocked(apiClient.listCategories).mockResolvedValue([
      { id: 11, title: 'General', description: '', tag: 'GEN', project_id: 5 },
    ]);
    vi.mocked(apiClient.listApplicability).mockResolvedValue([
      { id: 12, title: 'All', description: '', tag: 'ALL', project_id: 5 },
    ]);
    vi.mocked(apiClient.listRequirementStatuses).mockResolvedValue([
      {
        id: 13,
        title: 'Draft',
        description: '',
        tag: 'Drf',
        project_id: 5,
        is_system: true,
        tag_color: null,
      },
    ]);
    vi.mocked(apiClient.listVerificationStatuses).mockResolvedValue([]);
    vi.mocked(apiClient.listVerificationMethodsByProject).mockResolvedValue([
      { id: 14, title: 'Test', description: '', tag: 'TEST', project_id: 5 },
    ]);
    vi.mocked(apiClient.listProjectMembers).mockResolvedValue([
      { user_id: 15, role: 1, role_label: 'Admin', username: 'owner', name: 'Owner' },
    ]);
  });

  it('shows mapping after a successful preview', async () => {
    vi.mocked(apiClient.previewExcelImport).mockResolvedValue({
      import_type: 'requirements',
      columns: [{ index: 0, name: 'Title', sample_value: 'Alpha requirement' }],
      sample_rows: [['Alpha requirement']],
      row_count: 1,
      available_fields: {
        requirements: ['title', 'description'],
        tests: ['name', 'description'],
      },
      unique_values: { Title: ['Alpha requirement'] },
    });

    const user = userEvent.setup();
    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/space-project/import']}>
          <Routes>
            <Route path="/:projectSlug/import" element={<ImportPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    expect(screen.getByRole('heading', { name: /import from excel/i })).toBeInTheDocument();
    const file = new File(['Title\nHello world\n'], 'reqs.csv', { type: 'text/csv' });
    await user.upload(screen.getByTestId('import-file'), file);
    await user.click(screen.getByRole('button', { name: /upload and map columns/i }));

    await waitFor(() =>
      expect(screen.getByRole('combobox', { name: /map title/i })).toBeInTheDocument(),
    );
    expect(apiClient.previewExcelImport).toHaveBeenCalled();
  });

  it('defaults unknown catalog values and sends the confirmed mapping', async () => {
    vi.mocked(apiClient.previewExcelImport).mockResolvedValue({
      import_type: 'requirements',
      columns: [
        { index: 0, name: 'Title', sample_value: 'Alpha requirement' },
        { index: 1, name: 'Category', sample_value: 'Alien category' },
      ],
      sample_rows: [['Alpha requirement', 'Alien category']],
      row_count: 1,
      available_fields: {
        requirements: ['title', 'category_id'],
        tests: ['name'],
      },
      unique_values: {
        Title: ['Alpha requirement'],
        Category: ['Alien category'],
      },
    });
    vi.mocked(apiClient.commitExcelImport).mockResolvedValue({
      success: true,
      message: 'Successfully imported 1 records',
      imported_count: 1,
      errors: [],
      imported_requirement_ids: [1],
    });

    const user = userEvent.setup();
    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/space-project/import']}>
          <Routes>
            <Route path="/:projectSlug/import" element={<ImportPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    const file = new File(
      ['Title,Category\nAlpha requirement,Alien category\n'],
      'reqs.csv',
      { type: 'text/csv' },
    );
    await user.upload(screen.getByTestId('import-file'), file);
    await user.click(screen.getByRole('button', { name: /upload and map columns/i }));

    const valueMapping = await screen.findByRole('combobox', {
      name: /map value alien category/i,
    });
    expect(valueMapping).toHaveValue('11');

    await user.click(screen.getByRole('button', { name: /^import$/i }));
    await waitFor(() => expect(apiClient.commitExcelImport).toHaveBeenCalled());
    await waitFor(() => expect(screen.queryByTestId('import-progress')).not.toBeInTheDocument());
    expect(apiClient.commitExcelImport).toHaveBeenCalledWith(
      5,
      file,
      'requirements',
      expect.any(Array),
      [
        {
          target_field: 'category_id',
          source_value: 'Alien category',
          target_id: 11,
        },
      ],
      'csrf-test',
    );
  });

  it('shows a progress indicator while import is running', async () => {
    vi.mocked(apiClient.previewExcelImport).mockResolvedValue({
      import_type: 'requirements',
      columns: [{ index: 0, name: 'Title', sample_value: 'Alpha requirement' }],
      sample_rows: [['Alpha requirement']],
      row_count: 2,
      available_fields: {
        requirements: ['title'],
        tests: ['name'],
      },
      unique_values: { Title: ['Alpha requirement'] },
    });

    let finishImport!: () => void;
    vi.mocked(apiClient.commitExcelImport).mockImplementation(
      () =>
        new Promise((resolve) => {
          finishImport = () =>
            resolve({
              success: true,
              message: 'Successfully imported 2 records',
              imported_count: 2,
              errors: [],
              imported_requirement_ids: [1, 2],
            });
        }),
    );

    const user = userEvent.setup();
    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={['/space-project/import']}>
          <Routes>
            <Route path="/:projectSlug/import" element={<ImportPage />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );

    const file = new File(['Title\nA\nB\n'], 'reqs.csv', { type: 'text/csv' });
    await user.upload(screen.getByTestId('import-file'), file);
    await user.click(screen.getByRole('button', { name: /upload and map columns/i }));
    await screen.findByRole('button', { name: /^import$/i });
    await user.click(screen.getByRole('button', { name: /^import$/i }));

    const progress = await screen.findByTestId('import-progress');
    expect(progress).toHaveTextContent(/importing 2 requirements/i);
    expect(screen.getByRole('progressbar', { name: /importing records/i })).toBeInTheDocument();

    finishImport();
    await waitFor(() => expect(screen.queryByTestId('import-progress')).not.toBeInTheDocument());
    expect(await screen.findByTestId('import-result')).toHaveTextContent('Imported 2 record(s).');
  });
});
