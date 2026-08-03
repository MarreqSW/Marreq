import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ThemeProvider } from '@/context/ThemeContext';
import * as apiClient from '@/api/client';
import GroupMembersPage from '../GroupMembersPage';

vi.mock('@/api/client');

vi.mock('@/context/DashboardContext', () => ({
  useDashboard: () => ({ csrfToken: 'csrf' }),
}));

function renderPage() {
  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={['/groups/3/members']}>
        <Routes>
          <Route path="/groups/:groupId/members" element={<GroupMembersPage />} />
        </Routes>
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe('GroupMembersPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(apiClient.getGroup).mockResolvedValue({
      id: 3,
      name: 'Avionics',
      slug: 'avionics',
      description: null,
      owner_id: 1,
      created_at: '',
      updated_at: '',
    });
    vi.mocked(apiClient.listGroupMembers).mockResolvedValue([
      { user_id: 1, role: 1, role_label: 'Owner' },
    ]);
    vi.mocked(apiClient.listUsersOptional).mockResolvedValue([
      {
        id: 1,
        username: 'alice',
        name: 'Alice',
        email: 'a@b.c',
        creation_date: '',
        last_login: '',
        is_admin: false,
      },
      {
        id: 2,
        username: 'bob',
        name: 'Bob',
        email: 'b@b.c',
        creation_date: '',
        last_login: '',
        is_admin: false,
      },
    ]);
  });

  it('adds a member and removes an existing one', async () => {
    vi.mocked(apiClient.setGroupMemberRole).mockResolvedValue({
      user_id: 2,
      role: 4,
      role_label: 'Viewer',
    });
    vi.mocked(apiClient.removeGroupMember).mockResolvedValue(undefined);
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    // After add, reload returns both members
    vi.mocked(apiClient.listGroupMembers)
      .mockResolvedValueOnce([{ user_id: 1, role: 1, role_label: 'Owner' }])
      .mockResolvedValue([
        { user_id: 1, role: 1, role_label: 'Owner' },
        { user_id: 2, role: 4, role_label: 'Viewer' },
      ]);

    renderPage();
    expect(await screen.findByText('Alice')).toBeInTheDocument();

    const selects = screen.getAllByRole('combobox');
    await userEvent.selectOptions(selects[0], '2');
    await userEvent.click(screen.getByRole('button', { name: /^add$/i }));
    await waitFor(() =>
      expect(apiClient.setGroupMemberRole).toHaveBeenCalledWith(3, 2, 4, 'csrf'),
    );

    expect(await screen.findByText('Bob')).toBeInTheDocument();
    const removeButtons = screen.getAllByRole('button', { name: /^remove$/i });
    await userEvent.click(removeButtons[removeButtons.length - 1]);
    await waitFor(() => expect(apiClient.removeGroupMember).toHaveBeenCalledWith(3, 2, 'csrf'));
  });
});
