import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { GroupResponse, Project, ProjectMember } from '@/api/types';
import ProjectGeneralSettings, { changedFields } from '../ProjectGeneralSettings';

vi.mock('@/api/client');

const project: Project = {
  id: 5,
  name: 'Space Project',
  slug: 'space-project',
  description: 'Satellite requirements',
  owner_id: 2,
  group_id: 10,
  status: 'Active',
  creation_date: null,
  update_date: null,
};

const group = (id: number, name: string): GroupResponse => ({
  id,
  name,
  slug: name.toLowerCase(),
  description: null,
  owner_id: 1,
  created_at: '',
  updated_at: '',
});

const members: ProjectMember[] = [
  { user_id: 2, role: 1, role_label: 'Admin', username: 'dr_smith', name: 'Dr Smith' },
  { user_id: 3, role: 3, role_label: 'Author', username: 'eng_jones', name: 'Eng Jones' },
];

const userLabel = (id: number) => ({ 2: 'Dr Smith', 3: 'Eng Jones' })[id] ?? `User #${id}`;

function renderSettings(canEdit = true, onSaved = vi.fn()) {
  render(
    <ProjectGeneralSettings
      projectId={5}
      members={members}
      userLabel={userLabel}
      canEdit={canEdit}
      csrfToken="csrf-test"
      onSaved={onSaved}
    />,
  );
  return onSaved;
}

beforeEach(() => {
  vi.mocked(apiClient.listProjectsOptional).mockReset().mockResolvedValue([project]);
  vi.mocked(apiClient.listCreatableGroups).mockReset().mockResolvedValue([group(10, 'Platform'), group(20, 'Payload')]);
  vi.mocked(apiClient.getGroup).mockReset();
  vi.mocked(apiClient.updateProject).mockReset();
});

describe('ProjectGeneralSettings', () => {
  it('is read-only without the manage-configuration permission', async () => {
    renderSettings(false);
    const name = await screen.findByLabelText('Name');
    expect(name).toBeDisabled();
    expect(screen.getByLabelText('Status')).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save changes' })).not.toBeInTheDocument();
    expect(screen.getByText(/Only project Admins and instance administrators/)).toBeInTheDocument();
    expect(screen.getByText('/space-project')).toBeInTheDocument();
  });

  it('sends only the changed fields and refreshes after saving', async () => {
    vi.mocked(apiClient.updateProject).mockResolvedValue({
      ...project,
      name: 'Space Project II',
      status: 'OnHold',
      project_base_path: '/space-project',
    });
    const user = userEvent.setup();
    const onSaved = renderSettings();
    const save = await screen.findByRole('button', { name: 'Save changes' });
    expect(save).toBeDisabled();

    await user.clear(screen.getByLabelText('Name'));
    await user.type(screen.getByLabelText('Name'), 'Space Project II');
    await user.selectOptions(screen.getByLabelText('Status'), 'OnHold');
    await user.click(save);

    expect(apiClient.updateProject).toHaveBeenCalledWith(5, { name: 'Space Project II', status: 'OnHold' }, 'csrf-test');
    expect(await screen.findByRole('status')).toHaveTextContent('Saved.');
    expect(onSaved).toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
  });

  it('clears the description, changes the owner and moves to the personal namespace', async () => {
    vi.mocked(apiClient.updateProject).mockResolvedValue({ ...project, project_base_path: '/space-project' });
    const user = userEvent.setup();
    renderSettings();
    await user.clear(await screen.findByLabelText('Description'));
    await user.selectOptions(screen.getByLabelText('Owner'), '3');
    await user.selectOptions(screen.getByLabelText('Group'), '');
    await user.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(apiClient.updateProject).toHaveBeenCalledWith(
      5,
      { description: null, owner_id: 3, group_id: null },
      'csrf-test',
    );
  });

  it('shows the current group even when the user cannot move projects into it', async () => {
    vi.mocked(apiClient.listCreatableGroups).mockResolvedValue([group(20, 'Payload')]);
    vi.mocked(apiClient.getGroup).mockResolvedValue(group(10, 'Platform'));
    renderSettings();
    await waitFor(() => expect(screen.getByLabelText('Group')).toHaveDisplayValue('Platform (platform)'));
    expect(apiClient.getGroup).toHaveBeenCalledWith(10);
  });

  it('validates the name and shows server errors', async () => {
    vi.mocked(apiClient.updateProject).mockRejectedValue(new Error('permission denied'));
    const user = userEvent.setup();
    renderSettings();
    const name = await screen.findByLabelText('Name');
    await user.clear(name);
    await user.type(name, 'X');
    expect(screen.getByText('Name must be at least 2 characters.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();

    await user.type(name, 'YZ');
    await user.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('permission denied');
  });

  it('reports when the project cannot be loaded', async () => {
    vi.mocked(apiClient.listProjectsOptional).mockResolvedValue(null);
    renderSettings();
    expect(await screen.findByText('Could not load the project properties.')).toBeInTheDocument();
  });
});

describe('changedFields', () => {
  const base = { name: 'A project', description: 'Text', status: 'Active' as const, ownerId: 2, groupId: 10 };
  it('is empty when nothing changed (ignoring surrounding whitespace)', () => {
    expect(changedFields(base, { ...base, name: ' A project ', description: 'Text ' })).toEqual({});
  });
  it('maps an emptied description to null and keeps group moves explicit', () => {
    expect(changedFields(base, { ...base, description: '', groupId: null })).toEqual({
      description: null,
      group_id: null,
    });
  });
});
