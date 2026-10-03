import { type KeyboardEvent, useEffect, useId, useRef, useState } from 'react';
import { StatusBadge } from '@/components/StatusBadge';

export type StatusOption = { id: number; title: string; tag_color: string | null };

type Props = {
  /** e.g. "requirement status", for accessible names. */
  category: string;
  options: StatusOption[];
  selected: number[];
  onToggle: (id: number) => void;
};

/**
 * "+ Add status" button and menu for one exact-status category (issue #361).
 * A `menuitemcheckbox` per status, so several can be ticked while it stays
 * open. Keyboard: ↑/↓, Home/End to move, Enter/Space to tick, Escape or Tab to
 * close (Escape returns focus to the button).
 */
export default function AddStatusMenu({ category, options, selected, onToggle }: Props) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const menuId = useId();
  const chosen = new Set(selected);

  useEffect(() => {
    if (!open) return;
    itemRefs.current[0]?.focus();
    const onDown = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open]);

  function close(returnFocus: boolean) {
    setOpen(false);
    if (returnFocus) buttonRef.current?.focus();
  }

  function onMenuKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    const items = itemRefs.current.filter((el): el is HTMLButtonElement => el != null);
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    const focus = (i: number) => items[(i + items.length) % items.length]?.focus();
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        focus(at + 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        focus(at - 1);
        break;
      case 'Home':
        e.preventDefault();
        focus(0);
        break;
      case 'End':
        e.preventDefault();
        focus(items.length - 1);
        break;
      case 'Escape':
        e.preventDefault();
        close(true);
        break;
      case 'Tab':
        close(false);
        break;
    }
  }

  return (
    <div className="relative" ref={rootRef}>
      <button
        ref={buttonRef}
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        aria-label={`Add ${category} filter`}
        onClick={() => setOpen((o) => !o)}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown' && !open) {
            e.preventDefault();
            setOpen(true);
          }
        }}
        className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium text-stitch-muted hover:bg-stitch-higher hover:text-stitch-fg focus:outline-hidden focus-visible:ring-2 focus-visible:ring-stitch-accent"
      >
        <span aria-hidden className="text-[13px] leading-none">
          +
        </span>
        Add status
      </button>
      {open ? (
        <div
          id={menuId}
          role="menu"
          aria-label={`${category[0].toUpperCase()}${category.slice(1)} filters`}
          onKeyDown={onMenuKeyDown}
          className="absolute left-0 top-[calc(100%+4px)] z-60 min-w-[200px] max-h-72 overflow-y-auto rounded-lg border border-stitch-border bg-stitch-surface py-1 shadow-stitch"
        >
          {options.map((st, i) => {
            const on = chosen.has(st.id);
            return (
              <button
                key={st.id}
                ref={(el) => {
                  itemRefs.current[i] = el;
                }}
                type="button"
                role="menuitemcheckbox"
                aria-checked={on}
                onClick={() => onToggle(st.id)}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-stitch-fg hover:bg-stitch-elevated focus:bg-stitch-elevated focus:outline-hidden"
              >
                <span
                  aria-hidden
                  className={`material-symbols-outlined text-base ${on ? 'text-stitch-accent' : 'text-transparent'}`}
                >
                  check
                </span>
                <StatusBadge title={st.title} tagColor={st.tag_color} />
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
