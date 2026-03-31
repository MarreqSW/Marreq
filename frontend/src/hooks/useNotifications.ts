import { useCallback, useEffect, useRef, useState } from 'react';
import type { Notification } from '../api/types';
import {
  getNotifications,
  getUnreadCount,
  markNotificationRead,
  markAllNotificationsRead,
  getCsrfToken,
} from '../api/client';

const POLL_INTERVAL_MS = 30_000;

export function useNotifications() {
  const [unreadCount, setUnreadCount] = useState(0);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [loading, setLoading] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refreshCount = useCallback(async () => {
    try {
      const count = await getUnreadCount();
      setUnreadCount(count);
    } catch {
      // silently ignore -- user may not be logged in
    }
  }, []);

  const refreshList = useCallback(async () => {
    setLoading(true);
    try {
      const items = await getNotifications(false, 30);
      setNotifications(items);
      const count = items.filter((n) => !n.read).length;
      setUnreadCount(count);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  const markRead = useCallback(
    async (id: number) => {
      try {
        const csrf = await getCsrfToken();
        await markNotificationRead(id, csrf);
        setNotifications((prev) =>
          prev.map((n) => (n.id === id ? { ...n, read: true } : n)),
        );
        setUnreadCount((c) => Math.max(0, c - 1));
      } catch {
        // ignore
      }
    },
    [],
  );

  const markAllRead = useCallback(async () => {
    try {
      const csrf = await getCsrfToken();
      await markAllNotificationsRead(csrf);
      setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
      setUnreadCount(0);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    refreshCount();

    const startPolling = () => {
      if (timerRef.current) return;
      timerRef.current = setInterval(() => {
        if (!document.hidden) {
          refreshCount();
        }
      }, POLL_INTERVAL_MS);
    };

    const handleVisibility = () => {
      if (!document.hidden) {
        refreshCount();
      }
    };

    startPolling();
    document.addEventListener('visibilitychange', handleVisibility);

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [refreshCount]);

  return {
    unreadCount,
    notifications,
    loading,
    refreshList,
    refreshCount,
    markRead,
    markAllRead,
  };
}
