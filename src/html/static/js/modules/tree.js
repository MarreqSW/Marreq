/**
 * Tree View Module - Minimalist
 * Simple hierarchical tree display with expand/collapse
 * 
 * @module tree
 */

import { on } from '../core/dom.js';

/**
 * Initialize tree controls
 * 
 * @public
 * @param {Object} config - Configuration object
 * @param {string} config.rootSelector - Selector for the tree root element
 * @param {string} [config.toggleSelector='[data-tree-toggle]'] - Selector for toggle buttons
 * @param {string} [config.branchSelector='[data-tree-branch]'] - Selector for branch containers
 * @param {string} [config.expandAllSelector='[data-tree-expand-all]'] - Selector for expand all button
 * @param {string} [config.collapseAllSelector='[data-tree-collapse-all]'] - Selector for collapse all button
 * @returns {Object|null} Public API object with tree control methods
 */
export function initTreeControls({
  rootSelector,
  toggleSelector = '[data-tree-toggle]',
  branchSelector = '[data-tree-branch]',
  expandAllSelector = '[data-tree-expand-all]',
  collapseAllSelector = '[data-tree-collapse-all]',
}) {
  const root = document.querySelector(rootSelector);
  if (!root) return null;

  const cleanup = [];

  // Toggle individual branch
  on(root, 'click', toggleSelector, (event, toggle) => {
    event.preventDefault();
    const nodeId = toggle.dataset.treeToggle;
    if (!nodeId) return;
    
    const branch = root.querySelector(`[data-tree-branch="${nodeId}"]`);
    if (!branch) return;
    
    toggleBranch(nodeId, root);
  });

  // Expand all branches
  const expandControl = document.querySelector(expandAllSelector);
  if (expandControl) {
    const expandHandler = (event) => {
      event.preventDefault();
      expandAll(root, branchSelector);
    };
    expandControl.addEventListener('click', expandHandler);
    cleanup.push(() => expandControl.removeEventListener('click', expandHandler));
  }

  // Collapse all branches
  const collapseControl = document.querySelector(collapseAllSelector);
  if (collapseControl) {
    const collapseHandler = (event) => {
      event.preventDefault();
      collapseAll(root, branchSelector);
    };
    collapseControl.addEventListener('click', collapseHandler);
    cleanup.push(() => collapseControl.removeEventListener('click', collapseHandler));
  }

  // Initialize with all branches collapsed
  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = true;
  });

  // Keyboard navigation
  const keyboardHandler = (event) => {
    handleKeyboardNavigation(event, root);
  };
  root.addEventListener('keydown', keyboardHandler);
  cleanup.push(() => root.removeEventListener('keydown', keyboardHandler));

  // Public API - minimal, most functionality accessed via DOM
  return {
    destroy: () => {
      cleanup.forEach(fn => fn());
      cleanup.length = 0;
    },
  };
}

/**
 * Toggle a single branch open/closed
 */
function toggleBranch(nodeId, root) {
  const branch = root.querySelector(`[data-tree-branch="${nodeId}"]`);
  const toggle = root.querySelector(`[data-tree-toggle="${nodeId}"]`);
  
  if (!branch || !toggle) return;

  const willExpand = branch.hidden;
  
  branch.hidden = !willExpand;
  toggle.setAttribute('aria-expanded', String(willExpand));

  const icon = toggle.querySelector('.c-tree__toggle-icon');
  if (icon) {
    icon.style.transform = willExpand ? 'rotate(90deg)' : 'rotate(0deg)';
  }
}

/**
 * Expand all branches in the tree
 */
function expandAll(root, branchSelector) {
  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = false;
    
    const nodeId = branch.dataset.treeBranch;
    const toggle = root.querySelector(`[data-tree-toggle="${nodeId}"]`);
    if (toggle) {
      toggle.setAttribute('aria-expanded', 'true');
      const icon = toggle.querySelector('.c-tree__toggle-icon');
      if (icon) icon.style.transform = 'rotate(90deg)';
    }
  });
}

/**
 * Collapse all branches in the tree
 */
function collapseAll(root, branchSelector) {
  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = true;
    
    const nodeId = branch.dataset.treeBranch;
    const toggle = root.querySelector(`[data-tree-toggle="${nodeId}"]`);
    if (toggle) {
      toggle.setAttribute('aria-expanded', 'false');
      const icon = toggle.querySelector('.c-tree__toggle-icon');
      if (icon) icon.style.transform = 'rotate(0deg)';
    }
  });
}



/**
 * Search tree and highlight matching nodes
 */
export function searchTree(root, query) {
  if (!root) return 0;

  const normalizedQuery = String(query || '').trim().toLowerCase();
  let matchCount = 0;

  root.querySelectorAll('.is-search-match').forEach(m => m.classList.remove('is-search-match'));

  if (!normalizedQuery) {
    root.querySelectorAll('[role="treeitem"]').forEach(n => n.classList.remove('is-filtered-out'));
    return 0;
  }

  root.querySelectorAll('[role="treeitem"]').forEach(node => {
    const matches = (node.dataset.searchText || '').toLowerCase().includes(normalizedQuery);

    if (matches) {
      matchCount++;
      const card = node.querySelector('.c-tree__requirement-card');
      if (card) card.classList.add('is-search-match');
      node.classList.remove('is-filtered-out');
      expandAncestors(node, root);
    } else {
      node.classList.add('is-filtered-out');
    }
  });

  return matchCount;
}

/**
 * Filter tree based on custom criteria
 */
export function filterTree(root, filterFn) {
  if (!root || typeof filterFn !== 'function') return 0;

  let visibleCount = 0;

  root.querySelectorAll('[role="treeitem"]').forEach(node => {
    if (filterFn(node)) {
      visibleCount++;
      node.classList.remove('is-filtered-out');
      expandAncestors(node, root);
    } else {
      node.classList.add('is-filtered-out');
    }
  });

  return visibleCount;
}

/**
 * Expand all ancestor branches for a given node
 */
function expandAncestors(node, root) {
  let current = node.parentElement;

  while (current && current !== root) {
    if (current.matches('[data-tree-branch]') && current.hidden) {
      const branchId = current.dataset.treeBranch;
      const toggle = root.querySelector(`[data-tree-toggle="${branchId}"]`);
      
      if (toggle) {
        current.hidden = false;
        toggle.setAttribute('aria-expanded', 'true');
        const icon = toggle.querySelector('.c-tree__toggle-icon');
        if (icon) icon.style.transform = 'rotate(90deg)';
      }
    }
    current = current.parentElement;
  }
}

/**
 * Focus a specific tree node by ID
 */
function focusTreeNode(root, nodeId) {
  if (!root || !nodeId) return;

  const node = root.querySelector(`[data-node-id="${nodeId}"]`);
  if (!node) return;

  expandAncestors(node, root);

  const card = node.querySelector('.c-tree__requirement-card');
  if (card) {
    card.focus();
    card.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }
}

/**
 * Handle keyboard navigation in the tree
 */
function handleKeyboardNavigation(event, root) {
  const { key } = event;
  const nodes = Array.from(root.querySelectorAll('[role="treeitem"]'))
    .filter(n => !n.classList.contains('is-filtered-out'));
  
  if (!nodes.length) return;

  const currentNode = event.target.closest('[role="treeitem"]');
  if (!currentNode) return;

  const currentIndex = nodes.indexOf(currentNode);

  switch (key) {
    case 'ArrowDown':
      event.preventDefault();
      if (currentIndex < nodes.length - 1) {
        const nextNode = nodes[currentIndex + 1];
        const card = nextNode.querySelector('.c-tree__requirement-card');
        card?.focus();
      }
      break;

    case 'ArrowUp':
      event.preventDefault();
      if (currentIndex > 0) {
        const prevNode = nodes[currentIndex - 1];
        const card = prevNode.querySelector('.c-tree__requirement-card');
        card?.focus();
      }
      break;

    case 'ArrowRight': {
      event.preventDefault();
      const toggle = currentNode.querySelector('[data-tree-toggle]');
      const isExpanded = currentNode.getAttribute('aria-expanded') === 'true';
      
      if (toggle && !isExpanded) {
        toggle.click();
      } else if (isExpanded) {
        const children = currentNode.querySelector('[data-tree-branch]');
        const firstChild = children?.querySelector('[role="treeitem"]');
        if (firstChild) {
          const card = firstChild.querySelector('.c-tree__requirement-card');
          card?.focus();
        }
      }
      break;
    }

    case 'ArrowLeft': {
      event.preventDefault();
      const isExpanded = currentNode.getAttribute('aria-expanded') === 'true';
      
      if (isExpanded) {
        const toggle = currentNode.querySelector('[data-tree-toggle]');
        toggle?.click();
      } else {
        const parentBranch = currentNode.closest('[data-tree-branch]');
        if (parentBranch) {
          const parentId = parentBranch.dataset.treeBranch;
          const parentNode = root.querySelector(`[data-node-id="${parentId}"]`);
          if (parentNode) {
            const card = parentNode.querySelector('.c-tree__requirement-card');
            card?.focus();
          }
        }
      }
      break;
    }

    case 'Enter':
    case ' ':
      event.preventDefault();
      const link = currentNode.querySelector('.c-tree__requirement-link');
      link?.click();
      break;

    case 'Home':
      event.preventDefault();
      if (nodes.length > 0) {
        const card = nodes[0].querySelector('.c-tree__requirement-card');
        card?.focus();
      }
      break;

    case 'End':
      event.preventDefault();
      if (nodes.length > 0) {
        const card = nodes[nodes.length - 1].querySelector('.c-tree__requirement-card');
        card?.focus();
      }
      break;
  }
}
