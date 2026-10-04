import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { clearDraft, writeDraft } from '@/utils/requirementDraft';

export type DraftSaveStatus = 'clean' | 'pending' | 'saved' | 'error';

export const DRAFT_DEBOUNCE_MS = 800;

type Options<V> = {
  /** Storage key; `null` until the page knows the user and has decided on any stored draft. */
  key: string | null;
  /** Whether the form differs from what is saved on the server. */
  dirty: boolean;
  values: V;
  baseVersionId: number | null;
  /** While true nothing is written or cleared (e.g. a stored draft is still on offer). */
  paused?: boolean;
};

/**
 * Keep a local draft of a dirty requirement form (issue #255): written
 * {@link DRAFT_DEBOUNCE_MS} after the last change, right away when the page is
 * hidden or left, and removed once the form is clean again.
 */
export function useDraftAutosave<V>({ key, dirty, values, baseVersionId, paused = false }: Options<V>) {
  const [status, setStatus] = useState<DraftSaveStatus>('clean');
  const [savedAt, setSavedAt] = useState<Date | null>(null);
  const serialized = useMemo(() => JSON.stringify(values), [values]);

  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  /** A change not written yet (kept apart from `timer`, which unmount cleanup cancels first). */
  const pending = useRef(false);
  const stopped = useRef(false);
  const latest = useRef({ key, dirty, paused, values, baseVersionId });
  latest.current = { key, dirty, paused, values, baseVersionId };

  const write = useCallback(() => {
    timer.current = null;
    pending.current = false;
    const { key: k, dirty: d, paused: p, values: v, baseVersionId: base } = latest.current;
    if (stopped.current || !k || p || !d) return;
    const at = new Date();
    if (writeDraft(k, { savedAt: at.toISOString(), baseVersionId: base, values: v })) {
      setSavedAt(at);
      setStatus('saved');
    } else {
      setStatus('error');
    }
  }, []);

  const flush = useCallback(() => {
    if (!pending.current) return;
    if (timer.current !== null) clearTimeout(timer.current);
    write();
  }, [write]);

  useEffect(() => {
    if (!key || paused || stopped.current) return;
    if (!dirty) {
      pending.current = false;
      clearDraft(key);
      setStatus('clean');
      return;
    }
    setStatus((s) => (s === 'error' ? s : 'pending'));
    pending.current = true;
    timer.current = setTimeout(write, DRAFT_DEBOUNCE_MS);
    return () => {
      if (timer.current !== null) clearTimeout(timer.current);
      timer.current = null;
    };
  }, [key, dirty, paused, serialized, write]);

  // Laptop lid, tab switch, reload or leaving the page: write what is pending now.
  useEffect(() => {
    const onHidden = () => {
      if (document.visibilityState === 'hidden') flush();
    };
    window.addEventListener('pagehide', flush);
    document.addEventListener('visibilitychange', onHidden);
    return () => {
      window.removeEventListener('pagehide', flush);
      document.removeEventListener('visibilitychange', onHidden);
      flush();
    };
  }, [flush]);

  // Only when no draft could be kept does leaving the page need a warning.
  useEffect(() => {
    if (status !== 'error' || !dirty) return;
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      e.preventDefault();
    };
    window.addEventListener('beforeunload', onBeforeUnload);
    return () => window.removeEventListener('beforeunload', onBeforeUnload);
  }, [status, dirty]);

  /** Drop the draft and stop writing (after a successful save, or on Cancel/Discard). */
  const discard = useCallback(() => {
    stopped.current = true;
    pending.current = false;
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
    if (latest.current.key) clearDraft(latest.current.key);
    setStatus('clean');
  }, []);

  /** Start writing again after {@link discard} (e.g. the page stays open after Save). */
  const resume = useCallback(() => {
    stopped.current = false;
  }, []);

  return { status, savedAt, discard, resume };
}
