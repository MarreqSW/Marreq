import { initTreeControls } from '../modules/tree.js';

export function init() {
  initTreeControls({
    rootSelector: '#RequirementsTree',
    toggleSelector: '[data-tree-toggle]',
    branchSelector: '[data-tree-branch]',
    expandAllSelector: '[data-tree-expand-all]',
    collapseAllSelector: '[data-tree-collapse-all]',
  });
}

