/**
 * Tree View Module with SVG Connectors
 * 
 * Features:
 * - Dynamic SVG-based connectors for parent-child relationships
 * - Automatic redraw on expand/collapse, resize, and filter
 * - Keyboard navigation support
 * - Search and filter capabilities
 * - Performance optimizations with RAF and batched DOM operations
 * - Memory-efficient SVG management with canvas reuse
 * 
 * @module tree
 * 
 * @example
 * // Basic usage
 * const api = initTreeControls({ rootSelector: '.c-tree' });
 * 
 * @example
 * // Search tree
 * api.search('requirement');
 * 
 * @example
 * // Filter with custom function
 * api.filter(node => node.dataset.status === 'active');
 * 
 * @example
 * // Manual redraw
 * api.redrawConnectors();
 * 
 * Debug Mode:
 *   Set DEBUG_CONNECTORS environment variable or URL param ?debug=connectors
 */

import { on } from '../core/dom.js';

/**
 * Constants
 */
const CONNECTOR_RADIUS = 12; // Corner radius for smooth SVG curves
const ANIMATION_DURATION = 300; // CSS animation duration in ms
const ANIMATION_BUFFER = 20; // Buffer time after animation
const REDRAW_DELAY = ANIMATION_DURATION + ANIMATION_BUFFER; // Total delay before redraw
const SVG_NAMESPACE = 'http://www.w3.org/2000/svg';

/**
 * Debug mode flag - checks URL params or environment
 * @type {boolean}
 */
const DEBUG_CONNECTORS = (() => {
  if (typeof URLSearchParams !== 'undefined' && typeof window !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    return params.get('debug') === 'connectors' || params.get('debug') === 'true';
  }
  return false;
})();

/**
 * Performance metrics tracking
 * @private
 */
const performanceMetrics = {
  lastRedrawTime: 0,
  redrawCount: 0,
  averageRedrawTime: 0,
};

/**
 * Log debug information about connector drawing
 * @private
 * @param {...*} args - Arguments to log
 * @returns {void}
 */
function debugLog(...args) {
  if (DEBUG_CONNECTORS && typeof console !== 'undefined') {
    console.log('[TreeConnectors]', ...args);
  }
}

/**
 * Validates if an element is a valid DOM node
 * @private
 * @param {*} element - Element to validate
 * @returns {boolean} True if valid DOM element
 */
function isValidElement(element) {
  return element instanceof HTMLElement || element instanceof SVGElement;
}

/**
 * Safely gets bounding rect with error handling
 * @private
 * @param {HTMLElement} element - Element to measure
 * @returns {DOMRect|null} Bounding rect or null if error
 */
function safeBoundingRect(element) {
  try {
    if (!isValidElement(element)) return null;
    return element.getBoundingClientRect();
  } catch (error) {
    debugLog('Error getting bounding rect:', error);
    return null;
  }
}

/**
 * Create or update SVG connectors for parent-child relationships
 * Optimized to batch layout reads and avoid recursion
 * 
 * @private
 * @param {HTMLElement} container - The tree container or children group
 * @returns {void}
 * 
 * Performance optimizations:
 * - Batch all getBoundingClientRect calls (read phase)
 * - Use DocumentFragment for path creation (write phase)
 * - Reuse existing SVG canvas instead of recreating
 * - Cache measurements in WeakMap for faster lookups
 */
function drawConnectors(container) {
  if (!isValidElement(container)) {
    debugLog('Invalid container provided to drawConnectors');
    return;
  }

  const startTime = performance.now();
  debugLog('Drawing connectors for container:', container);

  // Find or create SVG canvas (reuse existing to avoid DOM thrashing)
  let svg = container.querySelector('.c-tree__connectors');
  if (!svg) {
    svg = document.createElementNS(SVG_NAMESPACE, 'svg');
    svg.classList.add('c-tree__connectors');
    svg.setAttribute('aria-hidden', 'true');
    svg.setAttribute('role', 'presentation');
    container.insertBefore(svg, container.firstChild);
    debugLog('Created new SVG canvas');
  }

  // Clear existing paths efficiently
  while (svg.firstChild) {
    svg.removeChild(svg.firstChild);
  }

  // Get container dimensions (single layout read)
  const containerRect = safeBoundingRect(container);
  if (!containerRect) {
    debugLog('Could not get container dimensions');
    return;
  }

  const width = Math.ceil(containerRect.width);
  const height = Math.ceil(containerRect.height);

  svg.setAttribute('width', width);
  svg.setAttribute('height', height);
  svg.setAttribute('viewBox', `0 0 ${width} ${height}`);

  debugLog('Canvas dimensions:', width, 'x', height);

  // Find all parent nodes with visible children in this container (no recursion - only direct children)
  const nodes = Array.from(
    container.querySelectorAll(':scope > .c-tree__node, :scope > .c-tree__child-node')
  );

  debugLog('Found', nodes.length, 'nodes in container');
  
  if (nodes.length === 0) {
    debugLog('No nodes found - container might be empty');
    const duration = performance.now() - startTime;
    updatePerformanceMetrics(duration);
    return;
  }

  // Collect all elements we need to measure FIRST (batch layout reads)
  const measurements = [];
  
  nodes.forEach((node) => {
    const childrenContainer = node.querySelector(':scope > .c-tree__children');
    if (!childrenContainer || childrenContainer.hidden) return;

    const children = Array.from(
      childrenContainer.querySelectorAll(':scope > .c-tree__node, :scope > .c-tree__child-node')
    );
    if (children.length === 0) return;

    const toggle = node.querySelector(':scope > .c-tree__node-content > .c-tree__toggle');
    if (!toggle) return;

    measurements.push({
      toggle,
      children: children.filter(child => !child.classList.contains('is-filtered-out'))
    });
  });

  // Batch read all getBoundingClientRect calls (critical for performance)
  const rects = new Map();
  measurements.forEach(({ toggle, children }) => {
    const toggleRect = safeBoundingRect(toggle);
    if (toggleRect) {
      rects.set(toggle, toggleRect);
    }
    
    children.forEach(child => {
      const childToggle = child.querySelector(':scope > .c-tree__node-content > .c-tree__toggle');
      if (childToggle) {
        const childRect = safeBoundingRect(childToggle);
        if (childRect) {
          rects.set(childToggle, childRect);
        }
      }
    });
  });

  // Now create all paths (batch writes using DocumentFragment)
  let connectorCount = 0;
  const fragment = document.createDocumentFragment();

  measurements.forEach(({ toggle, children }) => {
    const toggleRect = rects.get(toggle);
    if (!toggleRect) return;

    const parentX = toggleRect.left - containerRect.left + toggleRect.width / 2;
    const parentY = toggleRect.top - containerRect.top + toggleRect.height / 2;

    debugLog('Parent node:', {
      x: parentX.toFixed(1),
      y: parentY.toFixed(1),
      children: children.length
    });

    children.forEach((child, index) => {
      const childToggle = child.querySelector(':scope > .c-tree__node-content > .c-tree__toggle');
      if (!childToggle) return;

      const childRect = rects.get(childToggle);
      if (!childRect) return;

      const childX = childRect.left - containerRect.left + childRect.width / 2;
      const childY = childRect.top - containerRect.top + childRect.height / 2;

      debugLog(`  Child ${index + 1}:`, {
        x: childX.toFixed(1),
        y: childY.toFixed(1)
      });

      // Create elbow connector path
      const path = createElbowPath(parentX, parentY, childX, childY);
      if (path) {
        fragment.appendChild(path);
        connectorCount++;
      }
    });
  });

  svg.appendChild(fragment);
  
  const duration = performance.now() - startTime;
  updatePerformanceMetrics(duration);
  
  debugLog(`Drew ${connectorCount} connectors in ${duration.toFixed(2)}ms`);
}

/**
 * Updates performance metrics
 * @private
 * @param {number} duration - Duration in milliseconds
 * @returns {void}
 */
function updatePerformanceMetrics(duration) {
  performanceMetrics.redrawCount++;
  performanceMetrics.lastRedrawTime = duration;
  
  // Calculate rolling average
  const count = performanceMetrics.redrawCount;
  const prevAvg = performanceMetrics.averageRedrawTime;
  performanceMetrics.averageRedrawTime = (prevAvg * (count - 1) + duration) / count;
}

/**
 * Create an SVG path element with an elbow connector
 * Draws a smooth path from parent toggle to child toggle
 * Path goes: VERTICAL down from parent, then ONE smooth curve to child
 * 
 * @private
 * @param {number} x1 - Parent X coordinate (toggle center)
 * @param {number} y1 - Parent Y coordinate (toggle center)
 * @param {number} x2 - Child X coordinate (toggle center)
 * @param {number} y2 - Child Y coordinate (toggle center)
 * @returns {SVGPathElement|null} SVG path element or null on error
 */
function createElbowPath(x1, y1, x2, y2) {
  try {
    // Validate inputs
    if (!Number.isFinite(x1) || !Number.isFinite(y1) || 
        !Number.isFinite(x2) || !Number.isFinite(y2)) {
      debugLog('Invalid coordinates for path:', { x1, y1, x2, y2 });
      return null;
    }

    const path = document.createElementNS(SVG_NAMESPACE, 'path');
    path.classList.add('c-tree__connector-path');

    const verticalDistance = Math.abs(y2 - y1);
    const horizontalDistance = Math.abs(x2 - x1);
    
    // Determine direction
    const goingDown = y2 > y1;
    const goingRight = x2 > x1;

    let d;

    // Use rounded corner if there's enough space
    if (horizontalDistance > CONNECTOR_RADIUS && verticalDistance > CONNECTOR_RADIUS) {
      // Path: Vertical down from parent, then ONE smooth curve to child toggle
      // The curve connects directly to the child at its vertical position
      d = `M ${x1} ${y1} L ${x1} ${y2 - (goingDown ? CONNECTOR_RADIUS : -CONNECTOR_RADIUS)} Q ${x1} ${y2} ${x1 + (goingRight ? CONNECTOR_RADIUS : -CONNECTOR_RADIUS)} ${y2} L ${x2} ${y2}`;
    } else {
      // Simple L-shaped path if too small for rounded corner
      d = `M ${x1} ${y1} L ${x1} ${y2} L ${x2} ${y2}`;
    }

    path.setAttribute('d', d);
    path.setAttribute('fill', 'none');
    path.setAttribute('vector-effect', 'non-scaling-stroke'); // Better rendering

    return path;
  } catch (error) {
    debugLog('Error creating path:', error);
    return null;
  }
}

/**
 * Redraw all connectors in the tree
 * Optimized to process all visible containers in a single pass
 * 
 * @private
 * @param {HTMLElement} root - Tree root element
 * @returns {void}
 */
function redrawAllConnectors(root) {
  if (!isValidElement(root)) {
    debugLog('ERROR: Invalid root element');
    return;
  }

  debugLog('=== Redrawing all connectors ===');

  try {
    // Collect all containers that need connectors (root + visible children containers)
    const containers = [];
    
    const rootContainer = root.querySelector('.c-tree__root');
    if (rootContainer) {
      containers.push(rootContainer);
    } else {
      debugLog('WARNING: No .c-tree__root found');
    }

    // Only get visible children containers
    const childContainers = root.querySelectorAll('.c-tree__children:not([hidden])');
    containers.push(...childContainers);
    
    debugLog('Found', containers.length, 'containers to process');
    
    // Process all containers - drawConnectors is now non-recursive
    containers.forEach((container, index) => {
      debugLog(`Processing container ${index + 1}/${containers.length}`);
      drawConnectors(container);
    });
    
    debugLog('=== Finished redrawing connectors ===');
  } catch (error) {
    debugLog('ERROR during redraw:', error);
  }
}

/**
 * Debounced redraw for resize events and multiple rapid toggles
 * Optimized to use single RAF and prevent redundant redraws
 * 
 * @private
 */
let redrawTimeout = null;

/**
 * Schedule a redraw on the next animation frame
 * Cancels any pending redraws to prevent redundant operations
 * 
 * @private
 * @param {HTMLElement} root - Tree root element
 * @returns {void}
 */
function scheduleRedraw(root) {
  if (!isValidElement(root)) {
    debugLog('Cannot schedule redraw: invalid root');
    return;
  }

  // Cancel any pending redraw
  if (redrawTimeout !== null) {
    cancelAnimationFrame(redrawTimeout);
  }
  
  // Schedule redraw on next animation frame (single RAF for speed)
  redrawTimeout = requestAnimationFrame(() => {
    redrawAllConnectors(root);
    redrawTimeout = null;
  });
}

/**
 * Initialize tree controls with search, filter, and keyboard navigation
 * 
 * @public
 * @param {Object} config - Configuration object
 * @param {string} config.rootSelector - Selector for the tree root element
 * @param {string} [config.toggleSelector='[data-tree-toggle]'] - Selector for toggle buttons
 * @param {string} [config.branchSelector='[data-tree-branch]'] - Selector for branch containers
 * @param {string} [config.expandAllSelector='[data-tree-expand-all]'] - Selector for expand all button
 * @param {string} [config.collapseAllSelector='[data-tree-collapse-all]'] - Selector for collapse all button
 * @returns {Object|null} Public API object with tree control methods, or null if root not found
 * 
 * @example
 * const treeApi = initTreeControls({
 *   rootSelector: '.c-tree',
 *   toggleSelector: '[data-tree-toggle]'
 * });
 * 
 * if (treeApi) {
 *   treeApi.search('requirement');
 *   treeApi.expandAll();
 * }
 */
export function initTreeControls({
  rootSelector,
  toggleSelector = '[data-tree-toggle]',
  branchSelector = '[data-tree-branch]',
  expandAllSelector = '[data-tree-expand-all]',
  collapseAllSelector = '[data-tree-collapse-all]',
}) {
  // Validate inputs
  if (typeof rootSelector !== 'string' || !rootSelector.trim()) {
    debugLog('ERROR: rootSelector must be a non-empty string');
    return null;
  }

  const root = document.querySelector(rootSelector);
  if (!root) {
    debugLog('ERROR: Root element not found:', rootSelector);
    return null;
  }

  debugLog('Initializing tree controls for:', rootSelector);

  // Store cleanup functions
  const cleanup = [];

  // State for keyboard navigation
  const treeNodes = () => Array.from(root.querySelectorAll('[role="treeitem"]'))
    .filter(n => !n.classList.contains('is-filtered-out'));

  // Toggle individual branch
  on(root, 'click', toggleSelector, (event, toggle) => {
    event.preventDefault();
    const nodeId = toggle.dataset.treeToggle;
    if (!nodeId) {
      debugLog('Toggle clicked but no nodeId found');
      return;
    }
    toggleBranch(nodeId, root);
    
    // Redraw connectors after the CSS animation completes
    setTimeout(() => scheduleRedraw(root), REDRAW_DELAY);
  });

  // Expand all branches
  const expandControl = document.querySelector(expandAllSelector);
  if (expandControl) {
    const expandHandler = (event) => {
      event.preventDefault();
      expandAll(root, branchSelector);
      setTimeout(() => scheduleRedraw(root), REDRAW_DELAY);
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
      setTimeout(() => scheduleRedraw(root), REDRAW_DELAY);
    };
    collapseControl.addEventListener('click', collapseHandler);
    cleanup.push(() => collapseControl.removeEventListener('click', collapseHandler));
  }

  // Initialize with all branches collapsed
  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = true;
    branch.setAttribute('aria-hidden', 'true');
  });

  // Initialize toggle icons
  updateAllToggleIcons(root);

  // Initial connector draw - wait for DOM to be fully rendered
  requestAnimationFrame(() => {
    debugLog('Initial connector draw');
    redrawAllConnectors(root);
  });

  // Redraw on window resize with ResizeObserver
  let resizeObserver = null;
  try {
    resizeObserver = new ResizeObserver(() => {
      scheduleRedraw(root);
    });
    resizeObserver.observe(root);
    cleanup.push(() => {
      if (resizeObserver) {
        resizeObserver.disconnect();
        resizeObserver = null;
      }
    });
  } catch (error) {
    debugLog('ResizeObserver not supported or error:', error);
  }

  // Keyboard navigation
  const keyboardHandler = (event) => {
    handleKeyboardNavigation(event, root, treeNodes);
  };
  root.addEventListener('keydown', keyboardHandler);
  cleanup.push(() => root.removeEventListener('keydown', keyboardHandler));

  // Public API
  const api = {
    /**
     * Expand all branches in the tree
     * @returns {void}
     */
    expandAll: () => {
      expandAll(root, branchSelector);
      setTimeout(() => scheduleRedraw(root), REDRAW_DELAY);
    },
    
    /**
     * Collapse all branches in the tree
     * @returns {void}
     */
    collapseAll: () => {
      collapseAll(root, branchSelector);
      setTimeout(() => scheduleRedraw(root), REDRAW_DELAY);
    },
    
    /**
     * Filter tree based on custom function
     * @param {Function} filterFn - Function that returns true to show node
     * @returns {number} Number of visible nodes
     */
    filter: (filterFn) => {
      const count = filterTree(root, filterFn);
      setTimeout(() => scheduleRedraw(root), REDRAW_DELAY);
      return count;
    },
    
    /**
     * Search tree and highlight matches
     * @param {string} query - Search query
     * @returns {number} Number of matches found
     */
    search: (query) => {
      const count = searchTree(root, query);
      setTimeout(() => scheduleRedraw(root), REDRAW_DELAY);
      return count;
    },
    
    /**
     * Focus a specific node by ID
     * @param {string} nodeId - Node ID to focus
     * @returns {void}
     */
    focusNode: (nodeId) => focusTreeNode(root, nodeId),
    
    /**
     * Manually trigger a redraw of all connectors
     * @returns {void}
     */
    redrawConnectors: () => redrawAllConnectors(root),
    
    /**
     * Get performance metrics
     * @returns {Object} Performance metrics object
     */
    getMetrics: () => ({ ...performanceMetrics }),
    
    /**
     * Clean up event listeners and observers
     * Call this when destroying the tree
     * @returns {void}
     */
    destroy: () => {
      debugLog('Destroying tree controls');
      cleanup.forEach(fn => {
        try {
          fn();
        } catch (error) {
          debugLog('Error during cleanup:', error);
        }
      });
      cleanup.length = 0;
    },
  };

  return api;
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
 * 
 * @public
 * @param {HTMLElement} root - Tree root element
 * @param {string} query - Search query
 * @returns {number} - Number of matches found
 * 
 * @example
 * const count = searchTree(treeElement, 'requirement');
 * console.log(`Found ${count} matches`);
 */
export function searchTree(root, query) {
  if (!isValidElement(root)) return 0;

  // Validate and normalize query
  if (query === null || query === undefined) {
    query = '';
  }
  
  if (typeof query !== 'string') {
    query = String(query);
  }

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
    scheduleRedraw(root);
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

  scheduleRedraw(root);
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

  scheduleRedraw(root);
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
