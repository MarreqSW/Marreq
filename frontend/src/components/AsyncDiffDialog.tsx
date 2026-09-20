import { type ReactNode, useEffect, useRef, useState } from 'react';

type AsyncDiffDialogProps<T> = {
  open: boolean;
  onClose: () => void;
  title: string;
  subtitle: string;
  load: () => Promise<T>;
  render: (value: T) => ReactNode;
};

export default function AsyncDiffDialog<T>({
  open,
  onClose,
  title,
  subtitle,
  load,
  render,
}: AsyncDiffDialogProps<T>) {
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const [value, setValue] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    let alive = true;
    setLoading(true);
    setError(null);
    setValue(null);
    load()
      .then((result) => {
        if (alive) setValue(result);
      })
      .catch((reason) => {
        if (alive) {
          setError(reason instanceof Error ? reason.message : 'Failed to load comparison');
        }
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [load, open]);

  useEffect(() => {
    if (!open) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    closeButtonRef.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [onClose, open]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[80] flex items-center justify-center bg-black/55 p-3 backdrop-blur-sm md:p-8"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="async-diff-title"
        className="flex max-h-full w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-stitch-border bg-stitch-surface shadow-2xl"
      >
        <header className="flex shrink-0 items-start justify-between gap-4 border-b border-stitch-border px-5 py-4 md:px-6">
          <div>
            <h2 id="async-diff-title" className="font-headline text-lg font-bold text-stitch-fg">
              {title}
            </h2>
            <p className="mt-1 text-xs text-stitch-muted">{subtitle}</p>
          </div>
          <button
            ref={closeButtonRef}
            type="button"
            aria-label="Close comparison"
            onClick={onClose}
            className="rounded-lg p-1.5 text-stitch-muted transition-colors hover:bg-stitch-elevated hover:text-stitch-fg"
          >
            <span className="material-symbols-outlined">close</span>
          </button>
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto p-5 md:p-6">
          {loading ? (
            <div className="py-12 text-center text-sm text-stitch-muted">Comparing versions…</div>
          ) : error ? (
            <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-900 dark:text-red-100">
              {error}
            </div>
          ) : value ? (
            render(value)
          ) : null}
        </div>
      </div>
    </div>
  );
}
