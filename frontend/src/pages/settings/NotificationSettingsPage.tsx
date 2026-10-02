import { useEffect, useState } from 'react';
import {
  deleteNotificationPreference,
  getNotificationPreferences,
  setNotificationPreference,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import { useSettingsContext } from './settingsContext';

/** Project settings › Notifications: the viewer's subscription to this project's events. */
export default function NotificationSettingsPage() {
  const { projectId: pid } = useSettingsContext();
  const { csrfToken } = useDashboard();
  const [notifInApp, setNotifInApp] = useState(false);
  const [notifEmail, setNotifEmail] = useState(false);
  const [notifBusy, setNotifBusy] = useState(true);

  useEffect(() => {
    let cancelled = false;
    Promise.resolve(getNotificationPreferences())
      .catch(() => [])
      .then((prefs) => {
        if (cancelled) return;
        const mine = (prefs ?? []).find((np) => np.project_id === pid);
        setNotifInApp(mine?.notify_in_app ?? false);
        setNotifEmail(mine?.notify_email ?? false);
        setNotifBusy(false);
      });
    return () => {
      cancelled = true;
    };
  }, [pid]);

  return (
    <div>
      {/* ── Notification preferences ───────────────────────────────────── */}
      <section>
        <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-4">Notifications</h3>
        <p className="text-sm text-stitch-muted mb-4">
          Subscribe to project events to receive notifications when requirements are created, updated, or deleted.
        </p>
        <div className="flex flex-col gap-3">
          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={notifInApp}
              disabled={notifBusy}
              className="h-4 w-4 rounded-sm border-stitch-border bg-stitch-bg text-stitch-accent focus:ring-stitch-accent"
              onChange={async (e) => {
                const checked = e.target.checked;
                setNotifInApp(checked);
                setNotifBusy(true);
                try {
                  if (!checked && !notifEmail) {
                    await deleteNotificationPreference(pid, csrfToken ?? '');
                  } else {
                    await setNotificationPreference(pid, { notify_in_app: checked, notify_email: notifEmail }, csrfToken ?? '');
                  }
                } catch {
                  setNotifInApp(!checked);
                } finally {
                  setNotifBusy(false);
                }
              }}
            />
            <span className="text-sm text-stitch-fg">In-app notifications for this project</span>
          </label>
          <label className="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={notifEmail}
              disabled={notifBusy}
              className="h-4 w-4 rounded-sm border-stitch-border bg-stitch-bg text-stitch-accent focus:ring-stitch-accent"
              onChange={async (e) => {
                const checked = e.target.checked;
                setNotifEmail(checked);
                setNotifBusy(true);
                try {
                  if (!checked && !notifInApp) {
                    await deleteNotificationPreference(pid, csrfToken ?? '');
                  } else {
                    await setNotificationPreference(pid, { notify_in_app: notifInApp, notify_email: checked }, csrfToken ?? '');
                  }
                } catch {
                  setNotifEmail(!checked);
                } finally {
                  setNotifBusy(false);
                }
              }}
            />
            <span className="text-sm text-stitch-fg">Email notifications for this project</span>
          </label>
        </div>
      </section>
    </div>
  );
}
