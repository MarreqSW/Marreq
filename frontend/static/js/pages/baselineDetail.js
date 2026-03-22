import { showRequirementDiff } from '../modules/requirementDiffModal.js';
import { showNotification } from '../modules/notifications.js';
import { jsonFetch } from '../core/net.js';

function init() {
  document.addEventListener('click', async (e) => {
    const trigger = e.target.closest('[data-action="show-baseline-diff"]');
    if (!trigger) return;
    e.preventDefault();
    const projectId = trigger.getAttribute('data-project-id');
    const baselineId = trigger.getAttribute('data-baseline-id');
    const reqId = trigger.getAttribute('data-req-id');
    if (!projectId || !baselineId || !reqId) return;
    try {
      const diff = await jsonFetch(
        `/api/projects/${projectId}/baselines/${baselineId}/requirements/${reqId}/diff/current`,
        { credentials: 'same-origin' }
      );
      showRequirementDiff(diff);
    } catch (err) {
      const msg = err?.payload?.message || err?.message || 'Failed to load diff';
      showNotification(msg, 'error');
    }
  });
}

export { init };
