import { useEffect, useState } from 'react';
import {
  type BackupInfo,
  downloadDatabaseBackup,
  getBackupInfo,
  getCsrfToken,
  getDeploymentInfo,
  listUsersOptional,
} from '@/api/client';
import type { DeploymentInfo } from '@/api/types';
import StitchPageHeader from '@/components/StitchPageHeader';
import { useDashboard } from '@/context/DashboardContext';
import { btnPrimary } from '@/pages/catalog/catalogUi';
import { parseUser } from '@/utils/parseUser';
import { ADMIN_BREADCRUMB } from '@/pages/admin/adminArea';
import { formatBytes } from '@/utils/formatBytes';


export default function BackupPage() {
  const { dashboard, csrfToken } = useDashboard();
  const me = parseUser(dashboard?.user);

  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [deployment, setDeployment] = useState<DeploymentInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [info, setInfo] = useState<BackupInfo | null>(null);
  const [includeAttachments, setIncludeAttachments] = useState(true);

  useEffect(() => {
    let alive = true;
    // `/api/users` is admin-only, so it doubles as the access check (same as System logs).
    listUsersOptional().then((users) => {
      if (!alive) return;
      setAllowed(users !== null);
      if (users === null) return;
      getBackupInfo()
        .then((i) => {
          if (alive) setInfo(i);
        })
        .catch(() => {
          if (alive) setInfo(null);
        });
    });
    getDeploymentInfo()
      .then((info) => {
        if (alive) setDeployment(info);
      })
      .catch(() => {
        if (alive) setDeployment(null);
      });
    return () => {
      alive = false;
    };
  }, []);

  const projectName = ADMIN_BREADCRUMB;
  const backupDisabled = deployment?.allows_database_backup === false;

  async function onDownload() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const attachments = includeAttachments && info?.attachments_available !== false;
      const filename = await downloadDatabaseBackup(csrfToken ?? (await getCsrfToken()), {
        attachments,
      });
      setNotice(`Downloaded ${filename}.`);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Backup failed');
    } finally {
      setBusy(false);
    }
  }

  if (allowed === null) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading…
      </div>
    );
  }

  if (!allowed) {
    return (
      <div>
        <StitchPageHeader
          projectName={projectName}
          section="Backup"
          title="Database backup"
          subtitle="Restricted area."
        />
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-center">
          <span className="material-symbols-outlined text-4xl text-stitch-muted mb-3 block">
            lock
          </span>
          <p className="text-stitch-fg font-semibold">Access denied</p>
          <p className="text-sm text-stitch-muted mt-2 max-w-md mx-auto">
            Database backups require a global administrator account. You are signed in as{' '}
            <span className="text-stitch-accent">{me?.username ?? '?'}</span>.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Backup"
        title="Database backup"
        subtitle="Download a full copy of the Marreq database and attachment files, e.g. before an upgrade."
      />

      {error ? (
        <div
          role="alert"
          className="mb-4 rounded-lg border border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100 text-sm px-4 py-2"
        >
          {error}
        </div>
      ) : null}
      {notice ? (
        <div
          role="status"
          className="mb-4 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-900 dark:text-emerald-100 text-sm px-4 py-2"
        >
          {notice}
        </div>
      ) : null}

      <div className="max-w-3xl space-y-4">
        {backupDisabled ? (
          <div
            className="rounded-xl border border-stitch-border bg-stitch-surface p-5 text-sm text-stitch-muted"
            data-testid="backup-disabled"
          >
            Database backups are not available in this deployment mode. Backups of the hosted
            service are managed by the hosting operator.
          </div>
        ) : (
          <div className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-3">
            <h3 className="text-sm font-bold text-stitch-accent uppercase tracking-wide">
              Full backup
            </h3>
            <p className="text-sm text-stitch-muted">
              Creates a <code>pg_dump</code> of the whole database (all projects, users, and audit
              logs) and downloads it, together with the attachment files, as{' '}
              <code>marreq-backup_&lt;date&gt;_&lt;time&gt;.tar.gz</code>. Nothing is kept on the
              server. The file contains password hashes and all project data, so store it
              securely.
            </p>
            {info?.attachments_available === false ? (
              <p className="text-sm text-stitch-muted" data-testid="backup-no-attachments">
                Attachment storage is not configured on this server, so the backup holds the
                database only.
              </p>
            ) : (
              <label className="flex items-start gap-2 text-sm text-stitch-fg">
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={includeAttachments}
                  onChange={(e) => setIncludeAttachments(e.target.checked)}
                  disabled={busy}
                />
                <span>
                  Include attachment files
                  {info
                    ? ` (${info.attachment_files} ${info.attachment_files === 1 ? 'file' : 'files'}, ${formatBytes(info.attachment_bytes)})`
                    : ''}
                  <span className="block text-xs text-stitch-muted">
                    Untick for a smaller, database-only backup; then back up the attachments volume
                    separately.
                  </span>
                </span>
              </label>
            )}
            <button
              type="button"
              className={btnPrimary}
              onClick={() => void onDownload()}
              disabled={busy}
            >
              {busy ? 'Generating backup…' : 'Download backup'}
            </button>
            {busy ? (
              <p className="text-xs text-stitch-muted">
                This can take a few minutes for large databases. Keep this page open.
              </p>
            ) : null}
          </div>
        )}

        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-2">
          <h3 className="text-sm font-bold text-stitch-accent uppercase tracking-wide">
            Restoring
          </h3>
          <p className="text-sm text-stitch-muted">
            Unpack the archive, restore <code>database.sql</code> into an empty database with{' '}
            <code>psql</code> from PostgreSQL 17 or newer, then copy <code>attachments/</code> into
            the attachments volume (see the database setup guide for the Docker commands):
          </p>
          <pre className="text-xs bg-stitch-elevated border border-stitch-border rounded-md p-3 overflow-x-auto text-stitch-fg">
            {`tar xzf marreq-backup_YYYYMMDD_HHMMSS.tar.gz
psql "$DATABASE_URL" < marreq-backup_YYYYMMDD_HHMMSS/database.sql
cp -a marreq-backup_YYYYMMDD_HHMMSS/attachments/. "$MARREQ_ATTACHMENTS_DIR"/`}
          </pre>
          <p className="text-xs text-stitch-muted">
            Downloads are recorded under System logs as an <code>EXPORT</code> entry.
          </p>
        </div>
      </div>
    </div>
  );
}
