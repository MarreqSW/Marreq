/**
 * Tests for requirements.js - Requirements list page functionality
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { init } from '../../../src/html/static/js/pages/requirements.js';

// Mock dependencies
vi.mock('../../../src/html/static/js/core/net.js', () => ({
  jsonFetch: vi.fn(),
  postJson: vi.fn(),
}));

vi.mock('../../../src/html/static/js/modules/notifications.js', () => ({
  showNotification: vi.fn(),
}));

describe('Requirements List Page', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
  });

  describe('Table Rendering', () => {
    it('should initialize with empty state when no table present', () => {
      document.body.innerHTML = '<div></div>';
      
      expect(() => init()).not.toThrow();
    });

    it('should collect rows from requirements table', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td class="reqman-requirements-row__cell--key">
                <div class="reqman-requirements-key">
                  <span class="reqman-requirements-key__value">REQ-SYS-001</span>
                  <button data-action="toggle-row-details" aria-controls="requirement-details-1" aria-expanded="false"></button>
                </div>
              </td>
              <td class="reqman-requirements-row__cell--title">
                <a href="/p/1/requirements/show/1" class="reqman-requirements-title">Test Requirement</a>
              </td>
              <td class="reqman-requirements-row__cell--status">
                <span class="reqman-requirements-status-badge" data-status="Draft">Draft</span>
              </td>
              <td class="reqman-requirements-row__cell--verification">Analysis</td>
              <td class="reqman-requirements-row__cell--updated"><time>2024-01-01</time></td>
              <td class="reqman-requirements-row__cell--author">Admin</td>
              <td class="reqman-requirements-row__cell--actions">
                <button data-action="duplicate-requirement" data-requirement-id="1">Duplicate</button>
              </td>
            </tr>
            <tr id="requirement-details-1" class="reqman-requirements-row__details" data-details-for="1" hidden>
              <td colspan="7">Details here</td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
        <button id="newRequirementButton">New</button>
      `;

      expect(() => init()).not.toThrow();
    });

    it('should decorate status badges with variants', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-1</span></td>
              <td><a class="reqman-requirements-title">Test</a></td>
              <td><span class="reqman-requirements-status-badge" data-status="Draft">Draft</span></td>
              <td><span class="reqman-requirements-row__cell--verification">Test</span></td>
              <td><time>2024-01-01</time></td>
              <td>Author</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const badge = document.querySelector('.reqman-requirements-status-badge');
      expect(badge.classList.contains('reqman-requirements-status-badge--draft')).toBe(true);
    });
  });

  describe('Search Functionality', () => {
    it('should filter rows based on search input', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-SYS-001</span></td>
              <td><a class="reqman-requirements-title">System Requirement</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr class="reqman-requirements-row" data-requirement-id="2" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-SYS-002</span></td>
              <td><a class="reqman-requirements-title">Network Requirement</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Testing</td>
              <td><time>2024-01-02</time></td>
              <td>User</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const searchInput = document.getElementById('requirementsSearch');
      searchInput.value = 'Network';
      searchInput.dispatchEvent(new Event('input'));

      // Wait for debounce
      setTimeout(() => {
        const rows = document.querySelectorAll('.reqman-requirements-row');
        expect(rows[0].classList.contains('is-filtered-out')).toBe(true);
        expect(rows[1].classList.contains('is-filtered-out')).toBe(false);
      }, 200);
    });

    it('should show all rows when search is cleared', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">Test</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const searchInput = document.getElementById('requirementsSearch');
      searchInput.value = 'NonExistent';
      searchInput.dispatchEvent(new Event('input'));

      setTimeout(() => {
        searchInput.value = '';
        searchInput.dispatchEvent(new Event('input'));

        setTimeout(() => {
          const row = document.querySelector('.reqman-requirements-row');
          expect(row.classList.contains('is-filtered-out')).toBe(false);
        }, 200);
      }, 200);
    });
  });

  describe('Sorting Functionality', () => {
    it('should sort rows when clicking sortable header', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <thead>
            <tr>
              <th data-sort-key="key" class="is-sortable">
                Key <span class="reqman-requirements-table__sort-indicator">↕</span>
              </th>
              <th data-sort-key="title">Title</th>
            </tr>
          </thead>
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="2" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-002</span></td>
              <td><a class="reqman-requirements-title">Second</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-02</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">First</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Testing</td>
              <td><time>2024-01-01</time></td>
              <td>User</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const header = document.querySelector('[data-sort-key="key"]');
      header.click();

      const rows = document.querySelectorAll('.reqman-requirements-row');
      const firstKey = rows[0].querySelector('.reqman-requirements-key__value').textContent;
      expect(firstKey).toBe('REQ-001');
    });

    it('should toggle sort order on repeated clicks', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <thead>
            <tr>
              <th data-sort-key="title">
                Title <span class="reqman-requirements-table__sort-indicator">↕</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">Alpha</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr class="reqman-requirements-row" data-requirement-id="2" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-002</span></td>
              <td><a class="reqman-requirements-title">Zeta</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Testing</td>
              <td><time>2024-01-02</time></td>
              <td>User</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const header = document.querySelector('[data-sort-key="title"]');
      
      // First click - ascending
      header.click();
      let firstTitle = document.querySelectorAll('.reqman-requirements-row')[0]
        .querySelector('.reqman-requirements-title').textContent;
      expect(firstTitle).toBe('Alpha');
      
      // Second click - descending
      header.click();
      firstTitle = document.querySelectorAll('.reqman-requirements-row')[0]
        .querySelector('.reqman-requirements-title').textContent;
      expect(firstTitle).toBe('Zeta');
    });
  });

  describe('Row Details Toggle', () => {
    it('should toggle row details when clicking toggle button', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td>
                <button data-action="toggle-row-details" aria-controls="requirement-details-1" aria-expanded="false">
                  Toggle
                </button>
              </td>
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">Test</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr id="requirement-details-1" class="reqman-requirements-row__details" data-details-for="1" hidden>
              <td colspan="7">Detailed information</td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const toggleButton = document.querySelector('[data-action="toggle-row-details"]');
      const detailsRow = document.getElementById('requirement-details-1');

      expect(detailsRow.hasAttribute('hidden')).toBe(true);
      expect(toggleButton.getAttribute('aria-expanded')).toBe('false');

      toggleButton.click();

      expect(detailsRow.hasAttribute('hidden')).toBe(false);
      expect(toggleButton.getAttribute('aria-expanded')).toBe('true');
    });
  });

  describe('Filter Form', () => {
    it('should render filter chips for active filters', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">Test</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm">
          <select name="status_filter" data-filter-control="status" data-filter-label="Status">
            <option value="">All</option>
            <option value="1" selected>Draft</option>
          </select>
          <button type="button" data-action="clear-filters">Clear</button>
        </form>
        <div id="requirementsFilterChips"></div>
      `;

      init();

      const chipsContainer = document.getElementById('requirementsFilterChips');
      expect(chipsContainer.children.length).toBeGreaterThan(0);
      expect(chipsContainer.hidden).toBe(false);
    });

    it('should clear all filters when clicking clear button', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">Test</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" value="test" />
        <form id="requirementsFilterForm">
          <select name="status_filter" data-filter-control="status" data-filter-label="Status">
            <option value="">All</option>
            <option value="1" selected>Draft</option>
          </select>
          <button type="button" data-action="clear-filters">Clear</button>
        </form>
        <div id="requirementsFilterChips"></div>
      `;

      init();

      const clearButton = document.querySelector('[data-action="clear-filters"]');
      const statusSelect = document.querySelector('[name="status_filter"]');
      const searchInput = document.getElementById('requirementsSearch');

      clearButton.click();

      expect(statusSelect.value).toBe('');
      expect(searchInput.value).toBe('');
    });
  });

  describe('Keyboard Shortcuts', () => {
    it('should focus search input when pressing /', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">Test</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
        <button id="newRequirementButton">New</button>
      `;

      init();

      const searchInput = document.getElementById('requirementsSearch');
      const event = new KeyboardEvent('keydown', { key: '/' });
      document.dispatchEvent(event);

      expect(document.activeElement).toBe(searchInput);
    });

    it('should not trigger shortcuts when typing in input fields', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="reqman-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="reqman-requirements-key__value">REQ-001</span></td>
              <td><a class="reqman-requirements-title">Test</a></td>
              <td><span class="reqman-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
          </tbody>
        </table>
        <input type="text" id="someInput" />
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
        <button id="newRequirementButton">New</button>
      `;

      init();

      const someInput = document.getElementById('someInput');
      someInput.focus();

      const searchInput = document.getElementById('requirementsSearch');
      const clickSpy = vi.spyOn(searchInput, 'focus');

      const event = new KeyboardEvent('keydown', { key: '/', bubbles: true });
      someInput.dispatchEvent(event);

      expect(clickSpy).not.toHaveBeenCalled();
    });
  });
});
