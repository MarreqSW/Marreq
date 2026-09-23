import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  confirmEditApprovedRequirement,
  consumeEditApprovedAck,
  EDIT_APPROVED_CONFIRM_MESSAGE,
  isApprovedRequirement,
  preventEditNavigationIfUnconfirmed,
  resetApprovedEditPromptsForTests,
  setEditApprovedAck,
} from '../confirmEditApprovedRequirement';

describe('confirmEditApprovedRequirement', () => {
  beforeEach(() => {
    sessionStorage.clear();
    resetApprovedEditPromptsForTests();
    vi.spyOn(window, 'confirm').mockReturnValue(true);
  });

  afterEach(() => {
    vi.mocked(window.confirm).mockRestore();
    sessionStorage.clear();
    resetApprovedEditPromptsForTests();
  });

  it('treats only approved as locked for the warning', () => {
    expect(isApprovedRequirement('approved')).toBe(true);
    expect(isApprovedRequirement('Approved')).toBe(true);
    expect(isApprovedRequirement('draft')).toBe(false);
    expect(isApprovedRequirement('reviewed')).toBe(false);
    expect(isApprovedRequirement(null)).toBe(false);
  });

  it('does not prompt for draft and does not set an ack', () => {
    expect(confirmEditApprovedRequirement('draft', 4)).toBe(true);
    expect(window.confirm).not.toHaveBeenCalled();
    expect(consumeEditApprovedAck(4)).toBe(false);
  });

  it('prompts for approved, sets ack on proceed, and consume is one-shot', () => {
    expect(confirmEditApprovedRequirement('approved', 4)).toBe(true);
    expect(window.confirm).toHaveBeenCalledWith(EDIT_APPROVED_CONFIRM_MESSAGE);
    expect(consumeEditApprovedAck(4)).toBe(true);
    expect(consumeEditApprovedAck(4)).toBe(false);
  });

  it('does not set ack when the user cancels', () => {
    vi.mocked(window.confirm).mockReturnValue(false);
    expect(confirmEditApprovedRequirement('approved', 4)).toBe(false);
    expect(consumeEditApprovedAck(4)).toBe(false);
  });

  it('prevents navigation only when the user cancels a primary click', () => {
    vi.mocked(window.confirm).mockReturnValue(false);
    const event = {
      preventDefault: vi.fn(),
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      button: 0,
    };
    preventEditNavigationIfUnconfirmed(event, 'approved', 4);
    expect(event.preventDefault).toHaveBeenCalled();
  });

  it('skips the prompt for modified clicks', () => {
    const event = {
      preventDefault: vi.fn(),
      metaKey: true,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      button: 0,
    };
    preventEditNavigationIfUnconfirmed(event, 'approved', 4);
    expect(window.confirm).not.toHaveBeenCalled();
    expect(event.preventDefault).not.toHaveBeenCalled();
  });

  it('lets consume see a manually set ack', () => {
    setEditApprovedAck(9);
    expect(consumeEditApprovedAck(9)).toBe(true);
  });
});
