import { useCallback, useEffect, useRef, useState } from 'react';
import { useNotifications } from '@/hooks/useNotifications';
import type { Notification } from '@/api/types';

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr + 'Z').getTime();
  const diff = Math.max(0, now - then);
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function notificationIcon(type: string): string {
  switch (type) {
    case 'review_assigned':
      return 'rate_review';
    case 'approval_requested':
      return 'approval';
    case 'comment_added':
      return 'comment';
    case 'requirement_created':
      return 'add_circle';
    case 'requirement_updated':
      return 'edit';
    case 'requirement_deleted':
      return 'delete';
    default:
      return 'notifications';
  }
}

export default function NotificationPanel() {
  const { unreadCount, notifications, loading, refreshList, markRead, markAllRead } =
    useNotifications();
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  const toggle = useCallback(() => {
    setOpen((prev) => {
      if (!prev) refreshList();
      return !prev;
    });
  }, [refreshList]);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    function handleEscape(e: KeyboardEvent) {
      if (e.key === 'Escape') setOpen(false);
    }
    if (open) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleEscape);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [open]);

  const handleClick = useCallback(
    (n: Notification) => {
      if (!n.read) markRead(n.id);
      setOpen(false);
    },
    [markRead],
  );

  return (
    <div className="relative" ref={panelRef}>
      <button
        type="button"
        className="hover:bg-stitch-elevated p-2 rounded-full transition-colors relative"
        title="Notifications"
        onClick={toggle}
      >
        <span className="material-symbols-outlined text-xl">notifications</span>
        {unreadCount > 0 && (
          <span className="absolute top-1 right-1 flex h-4 min-w-[1rem] items-center justify-center rounded-full bg-red-500 px-1 text-[10px] font-bold text-white">
            {unreadCount > 99 ? '99+' : unreadCount}
          </span>
        )}
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-2 w-96 max-h-[28rem] overflow-y-auto rounded-lg border border-stitch-border bg-stitch-bg shadow-xl z-50">
          <div className="sticky top-0 flex items-center justify-between border-b border-stitch-border bg-stitch-bg px-4 py-3">
            <h3 className="text-sm font-semibold text-stitch-fg">Notifications</h3>
            {unreadCount > 0 && (
              <button
                type="button"
                className="text-xs text-stitch-accent hover:underline"
                onClick={markAllRead}
              >
                Mark all as read
              </button>
            )}
          </div>

          {loading && notifications.length === 0 && (
            <div className="px-4 py-8 text-center text-sm text-stitch-muted">Loading...</div>
          )}

          {!loading && notifications.length === 0 && (
            <div className="px-4 py-8 text-center text-sm text-stitch-muted">
              No notifications yet
            </div>
          )}

          {notifications.map((n) => (
            <button
              key={n.id}
              type="button"
              className={`flex w-full items-start gap-3 px-4 py-3 text-left transition-colors hover:bg-stitch-elevated ${
                !n.read ? 'bg-stitch-elevated/50' : ''
              }`}
              onClick={() => handleClick(n)}
            >
              <span className="material-symbols-outlined text-lg mt-0.5 shrink-0 text-stitch-muted">
                {notificationIcon(n.notification_type)}
              </span>
              <div className="min-w-0 flex-1">
                <p className={`text-sm leading-tight ${!n.read ? 'font-semibold text-stitch-fg' : 'text-stitch-muted'}`}>
                  {n.title}
                </p>
                {n.body && (
                  <p className="mt-0.5 text-xs text-stitch-muted truncate">{n.body}</p>
                )}
                <p className="mt-1 text-xs text-stitch-muted/70">{timeAgo(n.created_at)}</p>
              </div>
              {!n.read && (
                <span className="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-stitch-accent" />
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
