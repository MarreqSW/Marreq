/**
 * Tests for requirement.js presenter - View model builders
 */

import { describe, it, expect } from 'vitest';
import {
  buildRequirementViewModel,
  statusBadge,
  verificationBadge,
  verificationPercent,
  solidity,
  reference,
  initials,
  purpose,
  notesAndAttachments,
  timeline,
  EMPTY_MESSAGE,
} from '@presenters/requirement.js';

describe('Requirement Presenter', () => {
  describe('statusBadge', () => {
    it('should return success variant for accepted status', () => {
      const badge = statusBadge('Accepted');
      expect(badge.variant).toBe('bg-success');
      expect(badge.label).toBe('Accepted');
    });

    it('should return success variant for finished status', () => {
      const badge = statusBadge('Finished');
      expect(badge.variant).toBe('bg-success');
    });

    it('should return secondary variant for draft status', () => {
      const badge = statusBadge('Draft');
      expect(badge.variant).toBe('bg-secondary');
    });

    it('should return danger variant for rejected status', () => {
      const badge = statusBadge('Rejected');
      expect(badge.variant).toBe('bg-danger');
    });

    it('should handle empty status', () => {
      const badge = statusBadge('');
      expect(badge.variant).toBe('bg-secondary');
    });

    it('should be case-insensitive', () => {
      const badge = statusBadge('ACCEPTED');
      expect(badge.variant).toBe('bg-success');
    });
  });

  describe('verificationPercent', () => {
    it('should calculate percentage correctly', () => {
      const percent = verificationPercent({ total: 10, passed: 8 });
      expect(percent).toBe(80);
    });

    it('should return 0 for zero total', () => {
      const percent = verificationPercent({ total: 0, passed: 0 });
      expect(percent).toBe(0);
    });

    it('should handle missing counts', () => {
      const percent = verificationPercent({});
      expect(percent).toBe(0);
    });

    it('should round to nearest integer', () => {
      const percent = verificationPercent({ total: 3, passed: 2 });
      expect(percent).toBe(67);
    });
  });

  describe('verificationBadge', () => {
    it('should return warning variant when no tests linked', () => {
      const badge = verificationBadge({ total: 0 }, 'Analysis');
      expect(badge.variant).toBe('bg-warning');
      expect(badge.state).toBe('No verifications linked yet');
    });

    it('should return primary variant when all tests passing', () => {
      const badge = verificationBadge({ total: 5, passed: 5, failed: 0, pending: 0 }, 'Testing');
      expect(badge.variant).toBe('bg-primary');
      expect(badge.state).toBe('All linked verifications are passing');
    });

    it('should return info variant when tests pending', () => {
      const badge = verificationBadge({ total: 5, passed: 3, failed: 0, pending: 2 }, 'Testing');
      expect(badge.variant).toBe('bg-info');
      expect(badge.state).toBe('Verification in progress');
    });

    it('should return danger variant when tests failing', () => {
      const badge = verificationBadge({ total: 5, passed: 3, failed: 2, pending: 0 }, 'Testing');
      expect(badge.variant).toBe('bg-danger');
      expect(badge.state).toBe('Verification needs attention');
    });
  });

  describe('solidity', () => {
    it('should return "Needs definition" for draft with no tests', () => {
      const result = solidity({ total: 0 }, 'Draft');
      expect(result.label).toBe('Needs definition');
      expect(result.variant).toBe('text-muted');
    });

    it('should return "Unverified" for non-draft with no tests', () => {
      const result = solidity({ total: 0 }, 'Accepted');
      expect(result.label).toBe('Unverified');
    });

    it('should return "Rock solid" when all tests pass', () => {
      const result = solidity({ total: 5, passed: 5, failed: 0, pending: 0 }, 'Accepted');
      expect(result.label).toBe('Rock solid');
      expect(result.variant).toBe('text-success');
    });

    it('should return "Under evaluation" when tests pending', () => {
      const result = solidity({ total: 5, passed: 3, failed: 0, pending: 2 }, 'Accepted');
      expect(result.label).toBe('Under evaluation');
      expect(result.variant).toBe('text-info');
    });

    it('should return "At risk" when tests failing', () => {
      const result = solidity({ total: 5, passed: 3, failed: 2, pending: 0 }, 'Accepted');
      expect(result.label).toBe('At risk');
      expect(result.variant).toBe('text-danger');
    });
  });

  describe('initials', () => {
    it('should return first letter of name', () => {
      expect(initials('John Doe')).toBe('J');
    });

    it('should return uppercase initial', () => {
      expect(initials('alice')).toBe('A');
    });

    it('should handle empty name', () => {
      expect(initials('')).toBe('?');
    });

    it('should handle null/undefined', () => {
      expect(initials(null)).toBe('?');
      expect(initials(undefined)).toBe('?');
    });

    it('should trim whitespace', () => {
      expect(initials('  Bob  ')).toBe('B');
    });
  });

  describe('reference', () => {
    it('should return req_reference when present', () => {
      const result = reference({ req_reference: 'REQ-SYS-001', req_id: 42 });
      expect(result).toBe('REQ-SYS-001');
    });

    it('should generate reference from ID when missing', () => {
      const result = reference({ req_id: 5 });
      expect(result).toBe('REQ-0005');
    });

    it('should handle empty reference', () => {
      const result = reference({ req_reference: '', req_id: 10 });
      expect(result).toBe('REQ-0010');
    });

    it('should pad ID to 4 digits', () => {
      const result = reference({ req_id: 1 });
      expect(result).toBe('REQ-0001');
    });
  });

  describe('purpose', () => {
    it('should extract first paragraph', () => {
      const text = 'First paragraph.\n\nSecond paragraph.';
      expect(purpose(text)).toBe('First paragraph.');
    });

    it('should return entire text if no paragraphs', () => {
      const text = 'Single line text';
      expect(purpose(text)).toBe('Single line text');
    });

    it('should handle empty description', () => {
      expect(purpose('')).toBe('');
    });

    it('should trim whitespace', () => {
      const text = '  Trimmed text  ';
      expect(purpose(text)).toBe('Trimmed text');
    });
  });

  describe('notesAndAttachments', () => {
    it('should return notes and attachments when link provided', () => {
      const result = notesAndAttachments('https://example.com/doc');
      expect(result.notes).toContain('https://example.com/doc');
      expect(result.attachments).toHaveLength(1);
      expect(result.attachments[0].href).toBe('https://example.com/doc');
    });

    it('should return default message when no link', () => {
      const result = notesAndAttachments('');
      expect(result.notes).toBe('No implementation notes recorded.');
      expect(result.attachments).toHaveLength(0);
    });

    it('should handle null/undefined', () => {
      const result = notesAndAttachments(null);
      expect(result.notes).toBe('No implementation notes recorded.');
    });
  });

  describe('timeline', () => {
    it('should create current version entry', () => {
      const result = timeline({
        requirement: {
          req_current_status: 'Draft',
          req_update_date: '2024-01-01',
          req_author: 'Admin',
        },
        historyEntries: [],
      });

      expect(result[0].version).toBe('v1');
      expect(result[0].is_current).toBe(true);
      expect(result[0].action).toBe('CURRENT');
    });

    it('should include history entries', () => {
      const result = timeline({
        requirement: { req_current_status: 'Draft', req_author: 'Admin' },
        historyEntries: [
          {
            username: 'User1',
            log: {
              action_type: 'UPDATE',
              description: 'Updated requirement',
              created_at: '2024-01-01',
            },
          },
        ],
      });

      expect(result).toHaveLength(2);
      expect(result[1].version).toBe('v1');
      expect(result[1].action).toBe('UPDATE');
    });

    it('should calculate version numbers correctly', () => {
      const result = timeline({
        requirement: { req_current_status: 'Draft', req_author: 'Admin' },
        historyEntries: [
          { username: 'User1', log: { action_type: 'UPDATE', created_at: '2024-01-02' } },
          { username: 'User2', log: { action_type: 'UPDATE', created_at: '2024-01-01' } },
        ],
      });

      expect(result[0].version).toBe('v3'); // Current
      expect(result[1].version).toBe('v2'); // First history entry
      expect(result[2].version).toBe('v1'); // Second history entry
    });
  });

  describe('buildRequirementViewModel', () => {
    it('should build complete view model', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_title: 'Test Requirement',
          req_reference: 'REQ-001',
          req_description: 'Description',
          req_current_status: 'Draft',
          req_verification: 'Analysis',
          req_author: 'Admin',
          req_category: 'Systems',
          req_applicability: 'All Platforms',
        },
        project_id: 1,
        verification: {
          counts: { total: 5, passed: 5, failed: 0, pending: 0 },
        },
      };

      const view = buildRequirementViewModel(canonical);

      expect(view).toBeTruthy();
      expect(view.reference).toBe('REQ-001');
      expect(view.status_badge.label).toBe('Draft');
      expect(view.chips).toHaveLength(2);
    });

    it('should handle missing requirement', () => {
      const result = buildRequirementViewModel({});
      expect(result).toBeTruthy();
    });

    it('should handle null input', () => {
      const result = buildRequirementViewModel(null);
      expect(result).toBeNull();
    });

    it('should handle missing verification data', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_title: 'Test',
        },
        project_id: 1,
      };

      const view = buildRequirementViewModel(canonical);
      expect(view.verification_summary.total).toBe(0);
    });

    it('should format chips correctly', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_category: 'Systems',
          req_applicability: 'Hardware Only',
        },
        project_id: 1,
      };

      const view = buildRequirementViewModel(canonical);
      expect(view.chips).toEqual([
        { label: 'Systems', type: 'category' },
        { label: 'Hardware Only', type: 'applicability' },
      ]);
    });

    it('should filter out empty chips', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_category: 'Systems',
          req_applicability: '',
        },
        project_id: 1,
      };

      const view = buildRequirementViewModel(canonical);
      expect(view.chips).toHaveLength(1);
      expect(view.chips[0].label).toBe('Systems');
    });

    it('should build metadata correctly', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_author: 'John Doe',
          req_reviewer: 'Jane Smith',
          req_creation_date: '2024-01-01',
          req_update_date: '2024-01-02',
        },
        project_id: 1,
      };

      const view = buildRequirementViewModel(canonical);
      expect(view.metadata.author.name).toBe('John Doe');
      expect(view.metadata.author.initial).toBe('J');
      expect(view.metadata.reviewer.name).toBe('Jane Smith');
      expect(view.metadata.reviewer.assigned).toBe(true);
    });

    it('should handle unassigned reviewer', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_author: 'Admin',
          req_reviewer: '',
        },
        project_id: 1,
      };

      const view = buildRequirementViewModel(canonical);
      expect(view.metadata.reviewer.assigned).toBe(false);
      expect(view.metadata.reviewer.name).toBe('');
    });

    it('should build body sections', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_justification: 'This is the rationale',
          req_link: 'https://example.com/doc',
        },
        project_id: 1,
      };

      const view = buildRequirementViewModel(canonical);
      expect(view.body_sections).toHaveLength(2);
      expect(view.body_sections[0].title).toBe('Rationale');
      expect(view.body_sections[0].content).toBe('This is the rationale');
    });

    it('should handle empty body sections', () => {
      const canonical = {
        requirement: {
          req_id: 1,
          req_justification: '',
        },
        project_id: 1,
      };

      const view = buildRequirementViewModel(canonical);
      const rationaleSection = view.body_sections.find(s => s.title === 'Rationale');
      // When justification is empty, a fallback message is shown, so empty is false
      expect(rationaleSection.empty).toBe(false);
      expect(rationaleSection.content).toContain('rationale');
    });
  });

  describe('EMPTY_MESSAGE constant', () => {
    it('should export EMPTY_MESSAGE', () => {
      expect(EMPTY_MESSAGE).toBeTruthy();
      expect(typeof EMPTY_MESSAGE).toBe('string');
    });
  });
});
