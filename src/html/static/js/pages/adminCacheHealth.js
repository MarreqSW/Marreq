let intervalId = null;

async function refreshHealthData() {
  try {
    const response = await fetch('/api/v1/cache/performance');
    const data = await response.json();

    document.getElementById('hit-rate').textContent = data.hit_rate_percentage;
    document.getElementById('total-requests').textContent = data.total_requests;
    document.getElementById('avg-access-time').textContent = data.average_access_time_ms;
    document.getElementById('memory-usage').textContent = data.memory_usage_mb;

    const efficiency = data.cache_efficiency;
    document.getElementById('efficiency-bar').style.width = `${efficiency}%`;
    document.getElementById('efficiency-text').textContent = `${efficiency}%`;

    const status = document.getElementById('connection-status');
    status.textContent = 'Connected';
    status.className = 'badge bg-success';
  } catch (error) {
    console.error('Failed to fetch health data:', error);
    const status = document.getElementById('connection-status');
    status.textContent = 'Disconnected';
    status.className = 'badge bg-danger';
  }
}

function startMonitoring() {
  stopMonitoring();
  refreshHealthData();
  intervalId = setInterval(refreshHealthData, 10000);
}

function stopMonitoring() {
  if (intervalId) {
    clearInterval(intervalId);
    intervalId = null;
  }
}

export function init() {
  startMonitoring();

  const refreshButton = document.querySelector('[data-action="refresh-health"]');
  if (refreshButton) {
    refreshButton.addEventListener('click', () => {
      refreshHealthData();
    });
  }

  document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      stopMonitoring();
    } else {
      startMonitoring();
    }
  });
}
