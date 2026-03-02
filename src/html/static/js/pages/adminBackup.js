function timestamp() {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  const hours = String(now.getHours()).padStart(2, '0');
  const minutes = String(now.getMinutes()).padStart(2, '0');
  const seconds = String(now.getSeconds()).padStart(2, '0');
  return `${year}${month}${day}_${hours}${minutes}${seconds}`;
}

export function init() {
  const backupForm = document.getElementById('backupForm');
  const backupButton = document.querySelector('[data-action="generate-backup"]');
  const logsLink = document.querySelector('[data-action="generate-logs-export"]');

  if (backupForm && backupButton) {
    backupButton.addEventListener('click', (event) => {
      event.preventDefault();
      const filename = `marreq-backup_${timestamp()}.sql`;
      backupForm.action = `/admin/backup/generate/${filename}`;
      backupForm.submit();
    });
  }

  if (logsLink) {
    logsLink.addEventListener('click', (event) => {
      const filename = `marreq-logs_${timestamp()}.json`;
      logsLink.href = `/export_logs?filename=${filename}`;
    });
  }
}

