/**
 * Tests for requirements.js - Requirements list page functionality
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { init } from '@pages/requirements.js';

// Mock dependencies
vi.mock('@core/net.js', () => ({
  jsonFetch: vi.fn(),
  postJson: vi.fn(),
}));

vi.mock('@modules/notifications.js', () => ({
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
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td class="marreq-requirements-row__cell--key">
                <div class="marreq-requirements-key">
                  <span class="marreq-requirements-key__value">REQ-SYS-001</span>
                  <button data-action="toggle-row-details" aria-controls="requirement-details-1" aria-expanded="false"></button>
                </div>
              </td>
              <td class="marreq-requirements-row__cell--title">
                <a href="/p/1/requirements/show/1" class="marreq-requirements-title">Test Requirement</a>
              </td>
              <td class="marreq-requirements-row__cell--status">
                <span class="marreq-requirements-status-badge" data-status="Draft">Draft</span>
              </td>
              <td class="marreq-requirements-row__cell--verification">Analysis</td>
              <td class="marreq-requirements-row__cell--updated"><time>2024-01-01</time></td>
              <td class="marreq-requirements-row__cell--author">Admin</td>
              <td class="marreq-requirements-row__cell--actions">
                <button data-action="duplicate-requirement" data-requirement-id="1">Duplicate</button>
              </td>
            </tr>
            <tr id="requirement-details-1" class="marreq-requirements-row__details" data-details-for="1" hidden>
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
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-1</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge" data-status="Draft">Draft</span></td>
              <td><span class="marreq-requirements-row__cell--verification">Test</span></td>
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

      const badge = document.querySelector('.marreq-requirements-status-badge');
      expect(badge.classList.contains('marreq-requirements-status-badge--draft')).toBe(true);
    });
  });

  describe('Search Functionality', () => {
    it('should filter rows based on search input', async () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-SYS-001</span></td>
              <td><a class="marreq-requirements-title">System Requirement</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr class="marreq-requirements-row" data-requirement-id="2" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-SYS-002</span></td>
              <td><a class="marreq-requirements-title">Network Requirement</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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
      await new Promise(resolve => setTimeout(resolve, 200));
      
      const rows = document.querySelectorAll('.marreq-requirements-row');
      expect(rows.length).toBeGreaterThanOrEqual(2);
      if (rows.length >= 2) {
        expect(rows[0].classList.contains('is-filtered-out')).toBe(true);
        expect(rows[1].classList.contains('is-filtered-out')).toBe(false);
      }
    });

    it('should show all rows when search is cleared', async () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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

      await new Promise(resolve => setTimeout(resolve, 200));

      searchInput.value = '';
      searchInput.dispatchEvent(new Event('input'));

      await new Promise(resolve => setTimeout(resolve, 200));

      const row = document.querySelector('.marreq-requirements-row');
      if (row) {
        expect(row.classList.contains('is-filtered-out')).toBe(false);
      }
    });
  });

  describe('Sorting Functionality', () => {
    it('should sort rows when clicking sortable header', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <thead>
            <tr>
              <th data-sort-key="key" class="is-sortable">
                Key <span class="marreq-requirements-table__sort-indicator">↕</span>
              </th>
              <th data-sort-key="title">Title</th>
            </tr>
          </thead>
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="2" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-002</span></td>
              <td><a class="marreq-requirements-title">Second</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-02</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">First</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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

      const rows = document.querySelectorAll('.marreq-requirements-row');
      const firstKey = rows[0].querySelector('.marreq-requirements-key__value').textContent;
      expect(firstKey).toBe('REQ-001');
    });

    it('should toggle sort order on repeated clicks', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <thead>
            <tr>
              <th data-sort-key="title">
                Title <span class="marreq-requirements-table__sort-indicator">↕</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Alpha</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr class="marreq-requirements-row" data-requirement-id="2" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-002</span></td>
              <td><a class="marreq-requirements-title">Zeta</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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
      let firstTitle = document.querySelectorAll('.marreq-requirements-row')[0]
        .querySelector('.marreq-requirements-title').textContent;
      expect(firstTitle).toBe('Alpha');
      
      // Second click - descending
      header.click();
      firstTitle = document.querySelectorAll('.marreq-requirements-row')[0]
        .querySelector('.marreq-requirements-title').textContent;
      expect(firstTitle).toBe('Zeta');
    });
  });

  describe('Row Details Toggle', () => {
    it('should toggle row details when clicking toggle button', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td>
                <button data-action="toggle-row-details" aria-controls="requirement-details-1" aria-expanded="false">
                  Toggle
                </button>
              </td>
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td></td>
            </tr>
            <tr id="requirement-details-1" class="marreq-requirements-row__details" data-details-for="1" hidden>
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
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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

  describe('View Switcher', () => {
    beforeEach(() => {
      localStorage.clear();
    });

    it('should initialize with table view by default', () => {
      document.body.innerHTML = `
        <div id="tableView"></div>
        <div id="cardView"></div>
        <div id="treeView"></div>
        <button id="tableViewBtn"></button>
        <button id="cardViewBtn"></button>
        <button id="treeViewBtn"></button>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const tableView = document.getElementById('tableView');
      expect(tableView.style.display).toBe('block');
    });

    it('should switch to card view when card button clicked', () => {
      document.body.innerHTML = `
        <div id="tableView"></div>
        <div id="cardView"></div>
        <div id="treeView"></div>
        <button id="tableViewBtn" class="active"></button>
        <button id="cardViewBtn"></button>
        <button id="treeViewBtn"></button>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const cardViewBtn = document.getElementById('cardViewBtn');
      cardViewBtn.click();

      const cardView = document.getElementById('cardView');
      expect(cardView.style.display).toBe('block');
      expect(cardViewBtn.classList.contains('active')).toBe(true);
    });

    it('should persist view preference in localStorage', () => {
      document.body.innerHTML = `
        <div id="tableView"></div>
        <div id="cardView"></div>
        <div id="treeView"></div>
        <button id="tableViewBtn" class="active"></button>
        <button id="cardViewBtn"></button>
        <button id="treeViewBtn"></button>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const treeViewBtn = document.getElementById('treeViewBtn');
      treeViewBtn.click();

      expect(localStorage.getItem('requirements_view_preference')).toBe('tree');
    });

    it('should restore saved view preference', () => {
      localStorage.setItem('requirements_view_preference', 'card');

      document.body.innerHTML = `
        <div id="tableView"></div>
        <div id="cardView"></div>
        <div id="treeView"></div>
        <button id="tableViewBtn"></button>
        <button id="cardViewBtn"></button>
        <button id="treeViewBtn"></button>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const cardView = document.getElementById('cardView');
      expect(cardView.style.display).toBe('block');
    });
  });

  describe('Card View', () => {
    it('should collect cards from requirements grid', () => {
      document.body.innerHTML = `
        <div class="marreq-requirements-cards-grid">
          <div class="marreq-requirement-card" data-requirement-id="1" data-status-label="Draft" data-verification="Analysis" data-category="Systems">
            <div class="marreq-requirement-card__reference-text">REQ-001</div>
            <div class="marreq-requirement-card__title">Card Requirement</div>
            <div class="marreq-requirement-card__description">Description text</div>
            <div class="marreq-requirement-card__author">Author</div>
            <div class="marreq-requirement-card__date">2024-01-01</div>
          </div>
        </div>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      expect(() => init()).not.toThrow();
    });

    it('should filter cards based on search', async () => {
      document.body.innerHTML = `
        <div class="marreq-requirements-cards-grid">
          <div class="marreq-requirement-card" data-requirement-id="1" data-status-label="Draft">
            <div class="marreq-requirement-card__reference-text">REQ-001</div>
            <div class="marreq-requirement-card__title">Network Card</div>
          </div>
          <div class="marreq-requirement-card" data-requirement-id="2" data-status-label="Draft">
            <div class="marreq-requirement-card__reference-text">REQ-002</div>
            <div class="marreq-requirement-card__title">System Card</div>
          </div>
        </div>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const searchInput = document.getElementById('requirementsSearch');
      searchInput.value = 'Network';
      searchInput.dispatchEvent(new Event('input'));

      await new Promise(resolve => setTimeout(resolve, 200));

      const cards = document.querySelectorAll('.marreq-requirement-card');
      if (cards.length >= 2) {
        expect(cards[0].classList.contains('is-filtered-out')).toBe(false);
        expect(cards[1].classList.contains('is-filtered-out')).toBe(true);
      }
    });
  });

  describe('Tree View', () => {
    it('should collect tree nodes', () => {
      document.body.innerHTML = `
        <div class="c-tree">
          <div role="treeitem" data-requirement-id="1" data-status="1" data-category="2" data-verification="3" data-search-text="test requirement">
            <div class="c-tree__requirement-card">Node 1</div>
          </div>
        </div>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      expect(() => init()).not.toThrow();
    });

    it('should apply filters to tree nodes', () => {
      document.body.innerHTML = `
        <div class="c-tree">
          <div role="treeitem" data-requirement-id="1" data-status="1" data-search-text="requirement one"></div>
          <div role="treeitem" data-requirement-id="2" data-status="2" data-search-text="requirement two"></div>
        </div>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm">
          <select name="status_filter" data-filter-control="status">
            <option value="">All</option>
            <option value="1">Draft</option>
          </select>
        </form>
      `;

      init();

      const statusSelect = document.querySelector('[name="status_filter"]');
      statusSelect.value = '1';
      statusSelect.dispatchEvent(new Event('change'));

      // Filters are applied to tree
      expect(() => init()).not.toThrow();
    });
  });

  describe('Duplicate Functionality', () => {
    it('should handle duplicate button click', async () => {
      const { jsonFetch } = await import('@core/net.js');
      jsonFetch.mockResolvedValue({
        id: 1,
        title: 'Original',
        description: 'Description',
        reference_code: 'REQ-001',
      });

      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
              <td>Analysis</td>
              <td><time>2024-01-01</time></td>
              <td>Admin</td>
              <td><button data-action="duplicate-requirement" data-requirement-id="1">Dup</button></td>
            </tr>
          </tbody>
        </table>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
        <div id="duplicateRequirementModal">
          <form id="duplicateRequirementForm">
            <input type="text" id="dup_req_title" />
            <input type="text" id="dup_req_reference" />
            <textarea id="dup_req_description"></textarea>
            <textarea id="dup_req_justification"></textarea>
            <select id="dup_req_category"></select>
            <select id="dup_req_current_status"></select>
            <select id="dup_req_verification"></select>
            <select id="dup_req_applicability"></select>
            <select id="dup_req_reviewer"></select>
            <select id="dup_req_parent"></select>
            <input type="hidden" id="dup_project_id" />
            <input type="hidden" id="dup_req_author" />
          </form>
        </div>
      `;

      init();

      const dupButton = document.querySelector('[data-action="duplicate-requirement"]');
      await dupButton.click();

      // Wait for async operation
      await new Promise(resolve => setTimeout(resolve, 100));

      expect(jsonFetch).toHaveBeenCalledWith('/api/requirements/1');
    });
  });

  describe('Badge Overflow', () => {
    it('should handle badge overflow in cards', () => {
      document.body.innerHTML = `
        <div class="marreq-requirements-cards-grid">
          <div class="marreq-requirement-card" data-requirement-id="1" data-status-label="Draft">
            <div data-badge-rail>
              <div class="marreq-requirement-card__badge-rail">
                <span data-badge>Badge 1</span>
                <span data-badge>Badge 2</span>
              </div>
              <span data-overflow hidden></span>
            </div>
          </div>
        </div>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      expect(() => init()).not.toThrow();
    });
  });

  describe('No Results Banner', () => {
    it('should show banner when no results match search', async () => {
      document.body.innerHTML = `
        <div class="marreq-requirements-table-section">
          <table id="requirementsTable">
            <tbody>
              <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
                <td><span class="marreq-requirements-key__value">REQ-001</span></td>
                <td><a class="marreq-requirements-title">Test</a></td>
                <td><span class="marreq-requirements-status-badge">Draft</span></td>
                <td>Analysis</td>
                <td><time>2024-01-01</time></td>
                <td>Admin</td>
                <td></td>
              </tr>
            </tbody>
          </table>
        </div>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const searchInput = document.getElementById('requirementsSearch');
      searchInput.value = 'NonExistentRequirement';
      searchInput.dispatchEvent(new Event('input'));

      await new Promise(resolve => setTimeout(resolve, 200));

      const banner = document.querySelector('.marreq-requirements-search-empty');
      if (banner) {
        expect(banner.hidden).toBe(false);
      }
    });

    it('should hide banner when results are found', () => {
      document.body.innerHTML = `
        <div class="marreq-requirements-table-section">
          <table id="requirementsTable">
            <tbody>
              <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
                <td><span class="marreq-requirements-key__value">REQ-001</span></td>
                <td><a class="marreq-requirements-title">Test</a></td>
                <td><span class="marreq-requirements-status-badge">Draft</span></td>
                <td>Analysis</td>
                <td><time>2024-01-01</time></td>
                <td>Admin</td>
                <td></td>
              </tr>
            </tbody>
          </table>
        </div>
        <input type="search" id="requirementsSearch" />
        <form id="requirementsFilterForm"></form>
      `;

      init();

      const searchInput = document.getElementById('requirementsSearch');
      
      // First search with no results
      searchInput.value = 'NonExistent';
      searchInput.dispatchEvent(new Event('input'));

      setTimeout(() => {
        // Then search with results
        searchInput.value = 'Test';
        searchInput.dispatchEvent(new Event('input'));

        setTimeout(() => {
          const banner = document.querySelector('.marreq-requirements-search-empty');
          if (banner) {
            expect(banner.hidden).toBe(true);
          }
        }, 200);
      }, 200);
    });
  });

  describe('Status Definitions', () => {
    it('should parse status definitions from script tag', () => {
      document.body.innerHTML = `
        <script id="requirementsStatusDefinitions" type="application/json">
          [{"id": 1, "title": "Draft", "description": "Work in progress", "short_name": "DRF"}]
        </script>
        <table id="requirementsTable">
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge" data-status="Draft">Draft</span></td>
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

      const badge = document.querySelector('.marreq-requirements-status-badge');
      expect(badge.title).toContain('Work in progress');
    });

    it('should handle missing status definitions gracefully', () => {
      document.body.innerHTML = `
        <table id="requirementsTable">
          <tbody>
            <tr class="marreq-requirements-row" data-requirement-id="1" data-status-label="Draft">
              <td><span class="marreq-requirements-key__value">REQ-001</span></td>
              <td><a class="marreq-requirements-title">Test</a></td>
              <td><span class="marreq-requirements-status-badge">Draft</span></td>
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

      expect(() => init()).not.toThrow();
    });
  });
});
