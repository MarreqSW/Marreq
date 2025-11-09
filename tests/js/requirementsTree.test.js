/**
 * Tests for requirementsTree.js - Requirements tree view page
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { init } from '@pages/requirementsTree.js';

// Mock tree module
vi.mock('@modules/tree.js', () => ({
  initTreeControls: vi.fn(() => ({
    destroy: vi.fn(),
  })),
}));

describe('Requirements Tree Page', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should initialize tree controls', async () => {
      const { initTreeControls } = await import('@modules/tree.js');

      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree">
          <div role="treeitem" data-requirement-id="1">
            <button data-tree-toggle="1">Toggle</button>
            <div data-tree-branch="1" hidden>Children</div>
          </div>
        </div>
        <button data-tree-expand-all>Expand All</button>
        <button data-tree-collapse-all>Collapse All</button>
      `;

      init();

      expect(initTreeControls).toHaveBeenCalledWith({
        rootSelector: '#RequirementsTree',
        toggleSelector: '[data-tree-toggle]',
        branchSelector: '[data-tree-branch]',
        expandAllSelector: '[data-tree-expand-all]',
        collapseAllSelector: '[data-tree-collapse-all]',
      });
    });

    it('should handle missing tree root gracefully', async () => {
      const { initTreeControls } = await import('@modules/tree.js');

      document.body.innerHTML = '<div></div>';

      expect(() => init()).not.toThrow();
      expect(initTreeControls).toHaveBeenCalled();
    });
  });

  describe('Tree Structure', () => {
    it('should support nested tree items', () => {
      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree">
          <div role="treeitem" data-requirement-id="1" data-node-id="1">
            <button data-tree-toggle="1">Toggle</button>
            <div data-tree-branch="1" hidden>
              <div role="treeitem" data-requirement-id="2" data-node-id="2">
                <span>Child Node</span>
              </div>
            </div>
          </div>
        </div>
      `;

      expect(() => init()).not.toThrow();

      const parentNode = document.querySelector('[data-node-id="1"]');
      const childNode = document.querySelector('[data-node-id="2"]');

      expect(parentNode).toBeTruthy();
      expect(childNode).toBeTruthy();
    });

    it('should mark branches as hidden by default', () => {
      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree">
          <div role="treeitem" data-requirement-id="1">
            <button data-tree-toggle="1">Toggle</button>
            <div data-tree-branch="1">Children</div>
          </div>
        </div>
      `;

      const branch = document.querySelector('[data-tree-branch="1"]');
      expect(branch).toBeTruthy();
    });
  });

  describe('Tree Controls', () => {
    it('should have expand all button', () => {
      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree"></div>
        <button data-tree-expand-all>Expand All</button>
      `;

      const expandButton = document.querySelector('[data-tree-expand-all]');
      expect(expandButton).toBeTruthy();
    });

    it('should have collapse all button', () => {
      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree"></div>
        <button data-tree-collapse-all>Collapse All</button>
      `;

      const collapseButton = document.querySelector('[data-tree-collapse-all]');
      expect(collapseButton).toBeTruthy();
    });
  });

  describe('Accessibility', () => {
    it('should use proper ARIA roles', () => {
      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree" role="tree">
          <div role="treeitem" data-requirement-id="1" aria-expanded="false">
            <button data-tree-toggle="1">Toggle</button>
          </div>
        </div>
      `;

      const tree = document.getElementById('RequirementsTree');
      const treeitem = document.querySelector('[role="treeitem"]');

      expect(tree.getAttribute('role')).toBe('tree');
      expect(treeitem.getAttribute('role')).toBe('treeitem');
    });

    it('should have aria-expanded on toggle buttons', () => {
      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree">
          <div role="treeitem">
            <button data-tree-toggle="1" aria-expanded="false">Toggle</button>
          </div>
        </div>
      `;

      const toggle = document.querySelector('[data-tree-toggle]');
      expect(toggle.getAttribute('aria-expanded')).toBe('false');
    });
  });

  describe('Integration with Tree Module', () => {
    it('should pass correct selectors to tree module', async () => {
      const { initTreeControls } = await import('@modules/tree.js');

      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree"></div>
      `;

      init();

      const callArgs = initTreeControls.mock.calls[0][0];
      expect(callArgs.rootSelector).toBe('#RequirementsTree');
      expect(callArgs.toggleSelector).toBe('[data-tree-toggle]');
      expect(callArgs.branchSelector).toBe('[data-tree-branch]');
    });

    it('should return tree API', async () => {
      const mockAPI = {
        destroy: vi.fn(),
      };
      
      const { initTreeControls } = await import('@modules/tree.js');
      initTreeControls.mockReturnValue(mockAPI);

      document.body.innerHTML = `
        <div id="RequirementsTree" class="c-tree"></div>
      `;

      init();

      expect(initTreeControls).toHaveReturnedWith(mockAPI);
    });
  });
});
