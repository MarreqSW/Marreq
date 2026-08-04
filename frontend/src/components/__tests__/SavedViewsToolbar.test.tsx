import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SavedViewsToolbar from '../SavedViewsToolbar';
import type { RequirementsQueryState } from '@/utils/savedViewDefinition';
import { defaultQueryState } from '@/utils/savedViewDefinition';

vi.mock('@/api/client', () => ({
  listSavedViews: vi.fn(),
  createSavedView: vi.fn(),
  updateSavedView: vi.fn(),
  deleteSavedView: vi.fn(),
}));

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf-test' }),
}));

import * as api from '@/api/client';

const query: RequirementsQueryState = {
  ...defaultQueryState(),
  statusFilter: 2,
  q: 'alpha',
};

describe('SavedViewsToolbar', () => {
  beforeEach(() => {
    vi.mocked(api.listSavedViews).mockResolvedValue([
      {
        id: 1,
        project_id: 10,
        owner_id: 5,
        name: 'Mine',
        description: null,
        visibility: 'private',
        definition: defaultQueryState() as unknown as never,
        locked: false,
        locked_at: null,
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-01T00:00:00',
      },
      {
        id: 2,
        project_id: 10,
        owner_id: 9,
        name: 'Team open',
        description: null,
        visibility: 'shared',
        definition: {
          version: 1,
          entity: 'requirements',
          filters: { status_id: 3, q: 'shared-q' },
          sort: { column: null, dir: 'asc' },
          columns: null,
          ui: { view_mode: 'list', page_size: 50 },
        },
        locked: false,
        locked_at: null,
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-01T00:00:00',
      },
    ]);
  });

  it('lists views and applies selection', async () => {
    const user = userEvent.setup();
    const onSelectView = vi.fn();
    const onApplyState = vi.fn();
    render(
      <SavedViewsToolbar
        projectId={10}
        currentUserId={5}
        query={query}
        selectedViewId={null}
        onSelectView={onSelectView}
        onApplyState={onApplyState}
      />,
    );

    await waitFor(() => expect(api.listSavedViews).toHaveBeenCalledWith(10));
    const select = await screen.findByLabelText('Saved views');
    await user.selectOptions(select, '2');
    expect(onSelectView).toHaveBeenCalled();
    expect(onApplyState).toHaveBeenCalledWith(
      expect.objectContaining({
        statusFilter: 3,
        q: 'shared-q',
        viewMode: 'list',
        pageSize: 50,
      }),
    );
  });

  it('saves current query as a new view', async () => {
    const user = userEvent.setup();
    vi.mocked(api.createSavedView).mockResolvedValue({
      id: 3,
      project_id: 10,
      owner_id: 5,
      name: 'New view',
      description: null,
      visibility: 'private',
      definition: {} as never,
      locked: false,
      locked_at: null,
      created_at: '2024-01-01T00:00:00',
      updated_at: '2024-01-01T00:00:00',
    });
    const onSelectView = vi.fn();
    render(
      <SavedViewsToolbar
        projectId={10}
        currentUserId={5}
        query={query}
        selectedViewId={null}
        onSelectView={onSelectView}
        onApplyState={vi.fn()}
      />,
    );
    await screen.findByLabelText('Saved views');
    await user.click(screen.getByRole('button', { name: /Save current as/i }));
    await user.type(screen.getByPlaceholderText(/Open items/i), 'New view');
    await user.click(screen.getByRole('button', { name: /^Save$/i }));
    await waitFor(() => expect(api.createSavedView).toHaveBeenCalled());
    expect(api.createSavedView).toHaveBeenCalledWith(
      10,
      expect.objectContaining({
        name: 'New view',
        visibility: 'private',
        definition: expect.objectContaining({
          filters: expect.objectContaining({ status_id: 2, q: 'alpha' }),
        }),
      }),
      'csrf-test',
    );
  });
});
