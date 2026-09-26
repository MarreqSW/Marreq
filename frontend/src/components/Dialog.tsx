import { type ReactNode, useEffect, useId } from 'react';

type DialogProps = {
  open: boolean;
  onClose: () => void;
  title: string;
  subtitle?: string;
  children: ReactNode;
};

/** Small modal shell (same overlay as the version diff dialogs); closes on Escape or backdrop click. */
export default function Dialog({ open, onClose, title, subtitle, children }: DialogProps) {
  const titleId = useId();

  useEffect(() => {
    if (!open) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
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
        aria-labelledby={titleId}
        className="flex max-h-full w-full max-w-lg flex-col overflow-hidden rounded-2xl border border-stitch-border bg-stitch-surface shadow-2xl"
      >
        <header className="flex shrink-0 items-start justify-between gap-4 border-b border-stitch-border px-5 py-4">
          <div>
            <h2 id={titleId} className="font-headline text-lg font-bold text-stitch-fg">
              {title}
            </h2>
            {subtitle ? <p className="mt-1 text-xs text-stitch-muted">{subtitle}</p> : null}
          </div>
          <button
            type="button"
            aria-label="Close dialog"
            onClick={onClose}
            className="rounded-lg p-1.5 text-stitch-muted transition-colors hover:bg-stitch-elevated hover:text-stitch-fg"
          >
            <span className="material-symbols-outlined">close</span>
          </button>
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto p-5">{children}</div>
      </div>
    </div>
  );
}
