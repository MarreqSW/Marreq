import { on } from '../core/dom.js';

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

  on(root, 'click', toggleSelector, (event, toggle) => {
    event.preventDefault();
    const nodeId = toggle.dataset.treeToggle;
    if (!nodeId) return;
    toggleBranch(nodeId, root);
  });

  const expandControl = document.querySelector(expandAllSelector);
  if (expandControl) {
    expandControl.addEventListener('click', (event) => {
      event.preventDefault();
      root.querySelectorAll(branchSelector).forEach((branch) => {
        branch.hidden = false;
        branch.style.display = 'block';
      });
      rotateIcons(root, 90);
    });
  }

  const collapseControl = document.querySelector(collapseAllSelector);
  if (collapseControl) {
    collapseControl.addEventListener('click', (event) => {
      event.preventDefault();
      root.querySelectorAll(branchSelector).forEach((branch) => {
        branch.hidden = true;
        branch.style.display = 'none';
      });
      rotateIcons(root, 0);
    });
  }

  root.querySelectorAll(branchSelector).forEach((branch) => {
    branch.hidden = true;
    branch.style.display = 'none';
  });
  rotateIcons(root, 0);
}

function toggleBranch(nodeId, root) {
  const branch = root.querySelector(`[data-tree-branch="${nodeId}"]`);
  if (!branch) return;

  const isHidden = branch.hidden || branch.style.display === 'none';
  branch.hidden = !isHidden;
  branch.style.display = isHidden ? 'block' : 'none';

  const icon = root.querySelector(`[data-tree-toggle="${nodeId}"] .toggle-icon`);
  if (icon) {
    icon.style.transform = isHidden ? 'rotate(90deg)' : 'rotate(0deg)';
  }
}

function rotateIcons(root, degrees) {
  root.querySelectorAll('.toggle-icon').forEach((icon) => {
    icon.style.transform = `rotate(${degrees}deg)`;
  });
}
