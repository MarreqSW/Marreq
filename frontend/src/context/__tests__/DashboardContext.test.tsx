import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import * as apiClient from '@/api/client';
import type { DashboardPayload } from '@/api/types';
import { readDraft, requirementDraftKey, writeDraft } from '@/utils/requirementDraft';
import { DashboardProvider, useDashboard } from '../DashboardContext';

vi.mock('@/api/client');

const wrapper = ({ children }: { children: ReactNode }) => <DashboardProvider>{children}</DashboardProvider>;

afterEach(() => {
  localStorage.clear();
  vi.resetAllMocks();
});

describe('DashboardContext sign-out', () => {
  // Issue #255: a shared browser keeps no unsaved requirement text after sign-out.
  it('removes the signed-out user’s requirement drafts', async () => {
    vi.mocked(apiClient.getDashboard).mockResolvedValue({
      user: { id: 7, username: 'author' },
      csrf_token: 'csrf',
    } as unknown as DashboardPayload);
    vi.mocked(apiClient.logoutJson).mockResolvedValue(undefined);
    const draft = { savedAt: new Date().toISOString(), baseVersionId: null, values: { title: 'x' } };
    writeDraft(requirementDraftKey(7, 5, 'new'), draft);
    writeDraft(requirementDraftKey(8, 5, 'new'), draft);

    const { result } = renderHook(() => useDashboard(), { wrapper });
    await act(() => result.current.refresh());
    await act(() => result.current.logout());

    expect(apiClient.logoutJson).toHaveBeenCalledWith('csrf');
    expect(readDraft(requirementDraftKey(7, 5, 'new'))).toBeNull();
    expect(readDraft(requirementDraftKey(8, 5, 'new'))).not.toBeNull();
  });
});
