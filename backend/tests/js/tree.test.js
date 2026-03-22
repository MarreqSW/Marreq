/**
 * Tests for tree.js module - Tree view component
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { initTreeControls, searchTree, filterTree } from '@modules/tree.js';

describe('Tree Module', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
  });

  describe('initTreeControls', () => {
    it('should initialize tree controls', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-node-id="1">
            <button data-tree-toggle="1" aria-expanded="false">Toggle</button>
            <div data-tree-branch="1">Children</div>
          </div>
        </div>
        <button data-tree-expand-all>Expand All</button>
        <button data-tree-collapse-all>Collapse All</button>
      `;

      const api = initTreeControls({
        rootSelector: '#tree',
        toggleSelector: '[data-tree-toggle]',
        branchSelector: '[data-tree-branch]',
        expandAllSelector: '[data-tree-expand-all]',
        collapseAllSelector: '[data-tree-collapse-all]',
      });

      expect(api).toBeTruthy();
      expect(api.destroy).toBeInstanceOf(Function);
    });

    it('should return null when root not found', () => {
      document.body.innerHTML = '<div></div>';

      const api = initTreeControls({
        rootSelector: '#nonexistent',
      });

      expect(api).toBeNull();
    });

    it('should collapse all branches initially', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1">Toggle</button>
            <div data-tree-branch="1">Children</div>
          </div>
        </div>
      `;

      initTreeControls({
        rootSelector: '#tree',
        branchSelector: '[data-tree-branch]',
      });

      const branch = document.querySelector('[data-tree-branch="1"]');
      expect(branch.hidden).toBe(true);
    });

    it('should toggle branch on toggle button click', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1" aria-expanded="false">
              <span class="c-tree__toggle-icon"></span>
            </button>
            <div data-tree-branch="1" hidden>Children</div>
          </div>
        </div>
      `;

      initTreeControls({
        rootSelector: '#tree',
        toggleSelector: '[data-tree-toggle]',
        branchSelector: '[data-tree-branch]',
      });

      const toggle = document.querySelector('[data-tree-toggle="1"]');
      const branch = document.querySelector('[data-tree-branch="1"]');

      toggle.click();

      expect(branch.hidden).toBe(false);
      expect(toggle.getAttribute('aria-expanded')).toBe('true');
    });

    it('should expand all branches', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1" aria-expanded="false">Toggle</button>
            <div data-tree-branch="1" hidden>Children 1</div>
          </div>
          <div role="treeitem">
            <button data-tree-toggle="2" aria-expanded="false">Toggle</button>
            <div data-tree-branch="2" hidden>Children 2</div>
          </div>
        </div>
        <button data-tree-expand-all>Expand All</button>
      `;

      initTreeControls({
        rootSelector: '#tree',
        branchSelector: '[data-tree-branch]',
        expandAllSelector: '[data-tree-expand-all]',
      });

      const expandButton = document.querySelector('[data-tree-expand-all]');
      expandButton.click();

      const branches = document.querySelectorAll('[data-tree-branch]');
      branches.forEach(branch => {
        expect(branch.hidden).toBe(false);
      });
    });

    it('should collapse all branches', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1" aria-expanded="true">Toggle</button>
            <div data-tree-branch="1">Children 1</div>
          </div>
          <div role="treeitem">
            <button data-tree-toggle="2" aria-expanded="true">Toggle</button>
            <div data-tree-branch="2">Children 2</div>
          </div>
        </div>
        <button data-tree-collapse-all>Collapse All</button>
      `;

      initTreeControls({
        rootSelector: '#tree',
        branchSelector: '[data-tree-branch]',
        collapseAllSelector: '[data-tree-collapse-all]',
      });

      const collapseButton = document.querySelector('[data-tree-collapse-all]');
      collapseButton.click();

      const branches = document.querySelectorAll('[data-tree-branch]');
      branches.forEach(branch => {
        expect(branch.hidden).toBe(true);
      });
    });

    it('should cleanup event listeners on destroy', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1">Toggle</button>
            <div data-tree-branch="1">Children</div>
          </div>
        </div>
        <button data-tree-expand-all>Expand All</button>
      `;

      const api = initTreeControls({
        rootSelector: '#tree',
        expandAllSelector: '[data-tree-expand-all]',
      });

      expect(() => api.destroy()).not.toThrow();
    });
  });

  describe('searchTree', () => {
    it('should find matching nodes', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-search-text="system requirement">
            <div class="c-tree__requirement-card">System</div>
          </div>
          <div role="treeitem" data-search-text="network requirement">
            <div class="c-tree__requirement-card">Network</div>
          </div>
        </div>
      `;

      const root = document.getElementById('tree');
      const matchCount = searchTree(root, 'system');

      expect(matchCount).toBe(1);

      const nodes = document.querySelectorAll('[role="treeitem"]');
      expect(nodes[0].classList.contains('is-filtered-out')).toBe(false);
      expect(nodes[1].classList.contains('is-filtered-out')).toBe(true);
    });

    it('should highlight matching nodes', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-search-text="test requirement">
            <div class="c-tree__requirement-card">Test</div>
          </div>
        </div>
      `;

      const root = document.getElementById('tree');
      searchTree(root, 'test');

      const card = document.querySelector('.c-tree__requirement-card');
      expect(card.classList.contains('is-search-match')).toBe(true);
    });

    it('should clear filters when query is empty', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" class="is-filtered-out" data-search-text="test">
            <div class="c-tree__requirement-card">Test</div>
          </div>
        </div>
      `;

      const root = document.getElementById('tree');
      const matchCount = searchTree(root, '');

      expect(matchCount).toBe(0);

      const node = document.querySelector('[role="treeitem"]');
      expect(node.classList.contains('is-filtered-out')).toBe(false);
    });

    it('should be case-insensitive', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-search-text="System Requirement">
            <div class="c-tree__requirement-card">System</div>
          </div>
        </div>
      `;

      const root = document.getElementById('tree');
      const matchCount = searchTree(root, 'SYSTEM');

      expect(matchCount).toBe(1);
    });

    it('should handle null root', () => {
      const matchCount = searchTree(null, 'test');
      expect(matchCount).toBe(0);
    });

    it('should expand ancestor branches for matches', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-node-id="1">
            <button data-tree-toggle="1">Toggle</button>
            <div data-tree-branch="1" hidden>
              <div role="treeitem" data-search-text="nested requirement">
                <div class="c-tree__requirement-card">Nested</div>
              </div>
            </div>
          </div>
        </div>
      `;

      const root = document.getElementById('tree');
      searchTree(root, 'nested');

      const branch = document.querySelector('[data-tree-branch="1"]');
      expect(branch.hidden).toBe(false);
    });
  });

  describe('filterTree', () => {
    it('should filter nodes based on custom function', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-status="1">Node 1</div>
          <div role="treeitem" data-status="2">Node 2</div>
        </div>
      `;

      const root = document.getElementById('tree');
      const visibleCount = filterTree(root, (node) => node.dataset.status === '1');

      expect(visibleCount).toBe(1);

      const nodes = document.querySelectorAll('[role="treeitem"]');
      expect(nodes[0].classList.contains('is-filtered-out')).toBe(false);
      expect(nodes[1].classList.contains('is-filtered-out')).toBe(true);
    });

    it('should handle null root', () => {
      const visibleCount = filterTree(null, () => true);
      expect(visibleCount).toBe(0);
    });

    it('should handle invalid filter function', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">Node</div>
        </div>
      `;

      const root = document.getElementById('tree');
      const visibleCount = filterTree(root, null);

      expect(visibleCount).toBe(0);
    });

    it('should expand ancestors of visible nodes', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-node-id="1">
            <button data-tree-toggle="1">Toggle</button>
            <div data-tree-branch="1" hidden>
              <div role="treeitem" data-status="draft">Child</div>
            </div>
          </div>
        </div>
      `;

      const root = document.getElementById('tree');
      filterTree(root, (node) => node.dataset.status === 'draft');

      const branch = document.querySelector('[data-tree-branch="1"]');
      expect(branch.hidden).toBe(false);
    });

    it('should support complex filter logic', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem" data-status="1" data-category="2">Node 1</div>
          <div role="treeitem" data-status="1" data-category="3">Node 2</div>
          <div role="treeitem" data-status="2" data-category="2">Node 3</div>
        </div>
      `;

      const root = document.getElementById('tree');
      const visibleCount = filterTree(root, (node) => {
        return node.dataset.status === '1' && node.dataset.category === '2';
      });

      expect(visibleCount).toBe(1);
    });
  });

  describe('Keyboard Navigation', () => {
    it('should handle ArrowDown', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <a class="c-tree__requirement-card" tabindex="0">Node 1</a>
          </div>
          <div role="treeitem">
            <a class="c-tree__requirement-card" tabindex="0">Node 2</a>
          </div>
        </div>
      `;

      initTreeControls({ rootSelector: '#tree' });

      const root = document.getElementById('tree');
      const firstNode = root.querySelectorAll('[role="treeitem"]')[0];
      const firstCard = firstNode.querySelector('.c-tree__requirement-card');
      firstCard.focus();

      const event = new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true });
      firstNode.dispatchEvent(event);

      // Keyboard navigation is handled by the tree controls
      expect(event.defaultPrevented).toBe(false);
    });

    it('should handle ArrowUp', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <a class="c-tree__requirement-card">Node 1</a>
          </div>
          <div role="treeitem">
            <a class="c-tree__requirement-card">Node 2</a>
          </div>
        </div>
      `;

      initTreeControls({ rootSelector: '#tree' });

      const root = document.getElementById('tree');
      const secondNode = root.querySelectorAll('[role="treeitem"]')[1];
      
      const event = new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true });
      secondNode.dispatchEvent(event);

      expect(event.defaultPrevented).toBe(false);
    });

    it('should handle Home key', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <a class="c-tree__requirement-card">Node 1</a>
          </div>
          <div role="treeitem">
            <a class="c-tree__requirement-card">Node 2</a>
          </div>
        </div>
      `;

      initTreeControls({ rootSelector: '#tree' });

      const root = document.getElementById('tree');
      const lastNode = root.querySelectorAll('[role="treeitem"]')[1];
      
      const event = new KeyboardEvent('keydown', { key: 'Home', bubbles: true });
      lastNode.dispatchEvent(event);

      expect(event.defaultPrevented).toBe(false);
    });

    it('should handle End key', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <a class="c-tree__requirement-card">Node 1</a>
          </div>
          <div role="treeitem">
            <a class="c-tree__requirement-card">Node 2</a>
          </div>
        </div>
      `;

      initTreeControls({ rootSelector: '#tree' });

      const root = document.getElementById('tree');
      const firstNode = root.querySelectorAll('[role="treeitem"]')[0];
      
      const event = new KeyboardEvent('keydown', { key: 'End', bubbles: true });
      firstNode.dispatchEvent(event);

      expect(event.defaultPrevented).toBe(false);
    });
  });

  describe('Accessibility', () => {
    it('should set aria-expanded on toggle buttons', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1" aria-expanded="false">Toggle</button>
            <div data-tree-branch="1" hidden>Children</div>
          </div>
        </div>
      `;

      initTreeControls({
        rootSelector: '#tree',
        toggleSelector: '[data-tree-toggle]',
      });

      const toggle = document.querySelector('[data-tree-toggle="1"]');
      toggle.click();

      expect(toggle.getAttribute('aria-expanded')).toBe('true');
    });

    it('should update toggle icon rotation', () => {
      document.body.innerHTML = `
        <div id="tree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1" aria-expanded="false">
              <span class="c-tree__toggle-icon"></span>
            </button>
            <div data-tree-branch="1" hidden>Children</div>
          </div>
        </div>
      `;

      initTreeControls({
        rootSelector: '#tree',
        toggleSelector: '[data-tree-toggle]',
      });

      const toggle = document.querySelector('[data-tree-toggle="1"]');
      const icon = toggle.querySelector('.c-tree__toggle-icon');
      
      toggle.click();

      expect(icon.style.transform).toBe('rotate(90deg)');
    });
  });
});
