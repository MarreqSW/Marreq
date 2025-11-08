import { on } from '../core/dom.js';

/**
 * Initialize tree controls with search, filter, and keyboard navigation
 * @param {Object} config - Configuration object
 * @param {string} config.rootSelector - Selector for the tree root element
 * @param {string} config.toggleSelector - Selector for toggle buttons
 * @param {string} config.branchSelector - Selector for branch containers
 * @param {string} config.expandAllSelector - Selector for expand all button
 * @param {string} config.collapseAllSelector - Selector for collapse all button
 */
export function initTreeControls({
  rootSelector,
  toggleSelector = '[data-tree-toggle]',
  branchSelector = '[data-tree-branch]',
  expandAllSelector = '[data-tree-expand-all]',
  collapseAllSelector = '[data-tree-collapse-all]',
}) {
  const root = document.querySelector(rootSelector);
  if (!root) {
    return;
  }

  // State for keyboard navigation
  let focusedNodeIndex = -1;
  const treeNodes = () => Array.from(root.querySelectorAll('[role="treeitem"]'));

  // Toggle individual branch
  on(root, 'click', toggleSelector, (event, toggle) => {
    event.preventDefault();
    const nodeId = toggle.dataset.treeToggle;
    if (!nodeId) return;
    toggleBranch(nodeId, root);
  });

  // Expand all branches
  const expandControl = document.querySelector(expandAllSelector);
  if (expandControl) {
    expandControl.addEventListener('click', (event) => {
      event.preventDefault();
      expandAll(root, branchSelector);
    });
  }

  // Collapse all branches
  const collapseControl = document.querySelector(collapseAllSelector);
  if (collapseControl) {
    collapseControl.addEventListener('click', (event) => {
      event.preventDefault();
      collapseAll(root, branchSelector);
    });
  }

  // Initialize with all branches collapsed
  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = true;
    branch.setAttribute('aria-hidden', 'true');
  });

  // Initialize toggle icons
  updateAllToggleIcons(root);

  // Keyboard navigation
  root.addEventListener('keydown', (event) => {
    handleKeyboardNavigation(event, root, treeNodes);
  });

  // Public API
  return {
    expandAll: () => expandAll(root, branchSelector),
    collapseAll: () => collapseAll(root, branchSelector),
    filter: (filterFn) => filterTree(root, filterFn),
    search: (query) => searchTree(root, query),
    focusNode: (nodeId) => focusTreeNode(root, nodeId),
  };
}

/**
 * Toggle a single branch open/closed
 */
function toggleBranch(nodeId, root) {
  const branch = root.querySelector(`[data-tree-branch="${nodeId}"]`);
  const toggle = root.querySelector(`[data-tree-toggle="${nodeId}"]`);
  
  if (!branch || !toggle) return;

  const isHidden = branch.hidden || branch.getAttribute('aria-hidden') === 'true';
  
  branch.hidden = !isHidden;
  branch.setAttribute('aria-hidden', String(!isHidden));
  toggle.setAttribute('aria-expanded', String(!isHidden));

  // Update parent node aria-expanded
  const parentNode = toggle.closest('[role="treeitem"]');
  if (parentNode) {
    parentNode.setAttribute('aria-expanded', String(!isHidden));
  }

  // Animate icon rotation
  updateToggleIcon(toggle, !isHidden);
}

/**
 * Expand all branches in the tree
 */
function expandAll(root, branchSelector) {
  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = false;
    branch.setAttribute('aria-hidden', 'false');
    
    const nodeId = branch.dataset.treeBranch;
    const toggle = root.querySelector(`[data-tree-toggle="${nodeId}"]`);
    if (toggle) {
      toggle.setAttribute('aria-expanded', 'true');
      updateToggleIcon(toggle, true);
    }

    const parentNode = toggle?.closest('[role="treeitem"]');
    if (parentNode) {
      parentNode.setAttribute('aria-expanded', 'true');
    }
  });
}

/**
 * Collapse all branches in the tree
 */
function collapseAll(root, branchSelector) {
  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = true;
    branch.setAttribute('aria-hidden', 'true');
    
    const nodeId = branch.dataset.treeBranch;
    const toggle = root.querySelector(`[data-tree-toggle="${nodeId}"]`);
    if (toggle) {
      toggle.setAttribute('aria-expanded', 'false');
      updateToggleIcon(toggle, false);
    }

    const parentNode = toggle?.closest('[role="treeitem"]');
    if (parentNode) {
      parentNode.setAttribute('aria-expanded', 'false');
    }
  });
}

/**
 * Update toggle icon rotation
 */
function updateToggleIcon(toggle, isExpanded) {
  const icon = toggle.querySelector('.c-tree__toggle-icon');
  if (icon) {
    icon.style.transform = isExpanded ? 'rotate(90deg)' : 'rotate(0deg)';
  }
}

/**
 * Update all toggle icons based on current state
 */
function updateAllToggleIcons(root) {
  root.querySelectorAll('[data-tree-toggle]').forEach((toggle) => {
    const expanded = toggle.getAttribute('aria-expanded') === 'true';
    updateToggleIcon(toggle, expanded);
  });
}

/**
 * Search tree and highlight matching nodes
 * @param {HTMLElement} root - Tree root element
 * @param {string} query - Search query
 * @returns {number} - Number of matches found
 */
export function searchTree(root, query) {
  if (!root) return 0;

  const normalizedQuery = query.trim().toLowerCase();
  let matchCount = 0;

  // Clear previous highlights
  root.querySelectorAll('.is-search-match').forEach((node) => {
    node.classList.remove('is-search-match');
  });

  if (!normalizedQuery) {
    // Show all nodes when search is cleared
    root.querySelectorAll('[role="treeitem"]').forEach((node) => {
      node.classList.remove('is-filtered-out');
    });
    return 0;
  }

  const allNodes = Array.from(root.querySelectorAll('[role="treeitem"]'));

  allNodes.forEach((node) => {
    const searchText = (node.dataset.searchText || '').toLowerCase();
    const matches = searchText.includes(normalizedQuery);

    if (matches) {
      matchCount++;
      // Highlight the matching card
      const card = node.querySelector('.c-tree__requirement-card');
      if (card) {
        card.classList.add('is-search-match');
      }
      
      // Show the node
      node.classList.remove('is-filtered-out');
      
      // Expand parent branches to reveal this node
      expandAncestors(node, root);
    } else {
      node.classList.add('is-filtered-out');
    }
  });

  return matchCount;
}

/**
 * Filter tree based on custom criteria
 * @param {HTMLElement} root - Tree root element
 * @param {Function} filterFn - Filter function that returns true to show node
 * @returns {number} - Number of visible nodes
 */
export function filterTree(root, filterFn) {
  if (!root || typeof filterFn !== 'function') return 0;

  let visibleCount = 0;
  const allNodes = Array.from(root.querySelectorAll('[role="treeitem"]'));

  allNodes.forEach((node) => {
    const shouldShow = filterFn(node);

    if (shouldShow) {
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
    if (current.matches('[data-tree-branch]')) {
      const branchId = current.dataset.treeBranch;
      const toggle = root.querySelector(`[data-tree-toggle="${branchId}"]`);
      
      if (toggle && current.hidden) {
        current.hidden = false;
        current.setAttribute('aria-hidden', 'false');
        toggle.setAttribute('aria-expanded', 'true');
        updateToggleIcon(toggle, true);

        const parentNode = toggle.closest('[role="treeitem"]');
        if (parentNode) {
          parentNode.setAttribute('aria-expanded', 'true');
        }
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

  // Expand ancestors to reveal the node
  expandAncestors(node, root);

  // Remove previous selection
  root.querySelectorAll('[aria-selected="true"]').forEach((n) => {
    n.setAttribute('aria-selected', 'false');
  });

  // Select and focus the node
  node.setAttribute('aria-selected', 'true');
  const card = node.querySelector('.c-tree__requirement-card');
  if (card) {
    card.focus();
    card.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }
}

/**
 * Handle keyboard navigation in the tree
 */
function handleKeyboardNavigation(event, root, getNodes) {
  const { key } = event;
  const nodes = getNodes();
  
  if (!nodes.length) return;

  const currentNode = event.target.closest('[role="treeitem"]');
  if (!currentNode) return;

  const currentIndex = nodes.indexOf(currentNode);

  switch (key) {
    case 'ArrowDown':
      event.preventDefault();
      focusNextNode(nodes, currentIndex);
      break;

    case 'ArrowUp':
      event.preventDefault();
      focusPreviousNode(nodes, currentIndex);
      break;

    case 'ArrowRight': {
      event.preventDefault();
      const toggle = currentNode.querySelector('[data-tree-toggle]');
      const isExpanded = currentNode.getAttribute('aria-expanded') === 'true';
      
      if (toggle && !isExpanded) {
        // Expand if collapsed
        toggle.click();
      } else if (isExpanded) {
        // Move to first child
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
        // Collapse if expanded
        const toggle = currentNode.querySelector('[data-tree-toggle]');
        toggle?.click();
      } else {
        // Move to parent
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
      // Follow the link in the card
      const link = currentNode.querySelector('.c-tree__requirement-link');
      link?.click();
      break;

    case 'Home':
      event.preventDefault();
      focusFirstNode(nodes);
      break;

    case 'End':
      event.preventDefault();
      focusLastNode(nodes);
      break;

    default:
      break;
  }
}

function focusNextNode(nodes, currentIndex) {
  const visibleNodes = nodes.filter((n) => !n.classList.contains('is-filtered-out'));
  const currentVisibleIndex = visibleNodes.indexOf(nodes[currentIndex]);
  
  if (currentVisibleIndex < visibleNodes.length - 1) {
    const nextNode = visibleNodes[currentVisibleIndex + 1];
    const card = nextNode.querySelector('.c-tree__requirement-card');
    card?.focus();
  }
}

function focusPreviousNode(nodes, currentIndex) {
  const visibleNodes = nodes.filter((n) => !n.classList.contains('is-filtered-out'));
  const currentVisibleIndex = visibleNodes.indexOf(nodes[currentIndex]);
  
  if (currentVisibleIndex > 0) {
    const prevNode = visibleNodes[currentVisibleIndex - 1];
    const card = prevNode.querySelector('.c-tree__requirement-card');
    card?.focus();
  }
}

function focusFirstNode(nodes) {
  const visibleNodes = nodes.filter((n) => !n.classList.contains('is-filtered-out'));
  if (visibleNodes.length > 0) {
    const card = visibleNodes[0].querySelector('.c-tree__requirement-card');
    card?.focus();
  }
}

function focusLastNode(nodes) {
  const visibleNodes = nodes.filter((n) => !n.classList.contains('is-filtered-out'));
  if (visibleNodes.length > 0) {
    const card = visibleNodes[visibleNodes.length - 1].querySelector('.c-tree__requirement-card');
    card?.focus();
  }
}
