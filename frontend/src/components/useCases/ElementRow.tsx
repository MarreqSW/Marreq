import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import StatementText from '@/components/StatementText';

export type RowMenuItem = {
  icon: string;
  label: string;
  onSelect: () => void;
  danger?: boolean;
  disabled?: boolean;
};

type Props = {
  id: string;
  /** Display label: `3`, `A3.1`; bullets show none. */
  label?: string;
  text: string;
  /** Steps show a numbered badge; preconditions and postconditions a bullet. */
  variant: 'step' | 'branch-step' | 'bullet';
  editing: boolean;
  /** Highlight as the origin of the selected branch. */
  highlighted?: boolean;
  /** Focus the text field on mount (a row just inserted). */
  autoFocus?: boolean;
  /** Commit new text. Returns false when nothing was saved. */
  onCommit?: (text: string) => void;
  /** Enter at the end of the text: commit and start a new row below. */
  onEnter?: (text: string) => void;
  /** Blur or Escape on an empty, never-saved row. */
  onCancelEmpty?: () => void;
  onMove?: (delta: -1 | 1) => void;
  onInsertBelow?: () => void;
  menu?: RowMenuItem[];
  /** Drag and drop within the list. */
  dragHandlers?: {
    onDragStart: () => void;
    onDragOver: (before: boolean) => void;
    onDrop: () => void;
    onDragEnd: () => void;
    dropIndicator: 'before' | 'after' | null;
  };
  trailing?: ReactNode;
};

/** One block of a use case document: read-only text, or an inline editor with a hover toolbar. */
export default function ElementRow({
  id,
  label,
  text,
  variant,
  editing,
  highlighted,
  autoFocus,
  onCommit,
  onEnter,
  onCancelEmpty,
  onMove,
  onInsertBelow,
  menu,
  dragHandlers,
  trailing,
}: Props) {
  const [value, setValue] = useState(text);
  const [menuOpen, setMenuOpen] = useState(false);
  const ref = useRef<HTMLTextAreaElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => setValue(text), [text]);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = '0px';
    el.style.height = `${el.scrollHeight}px`;
  }, [value, editing]);

  useEffect(() => {
    if (autoFocus) ref.current?.focus();
  }, [autoFocus]);

  useEffect(() => {
    if (!menuOpen) return;
    const onDown = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node)) setMenuOpen(false);
    };
    window.addEventListener('mousedown', onDown);
    return () => window.removeEventListener('mousedown', onDown);
  }, [menuOpen]);

  function commit() {
    const next = value.trim();
    if (!next) {
      if (onCancelEmpty) onCancelEmpty();
      else setValue(text);
      return;
    }
    if (next !== text) onCommit?.(next);
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      const next = value.trim();
      if (next && onEnter) onEnter(next);
      else ref.current?.blur();
    } else if (e.key === 'Escape') {
      setValue(text);
      if (!text && onCancelEmpty) onCancelEmpty();
      ref.current?.blur();
    } else if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown') && onMove) {
      e.preventDefault();
      onMove(e.key === 'ArrowUp' ? -1 : 1);
    }
  }

  const badge =
    variant === 'bullet' ? (
      <span className="mt-[9px] h-1.5 w-1.5 shrink-0 rounded-full bg-stitch-muted" aria-hidden />
    ) : (
      <span
        className={`mt-0.5 flex h-6 min-w-6 shrink-0 items-center justify-center rounded-full px-1 text-[11px] font-bold ${
          variant === 'step'
            ? 'bg-stitch-accent text-stitch-on-accent'
            : 'bg-stitch-elevated font-mono text-stitch-fg'
        }`}
      >
        {label}
      </span>
    );

  const drop = dragHandlers?.dropIndicator;

  return (
    <div
      data-element-id={id}
      onDragOver={
        dragHandlers
          ? (e) => {
              e.preventDefault();
              const box = e.currentTarget.getBoundingClientRect();
              dragHandlers.onDragOver(e.clientY < box.top + box.height / 2);
            }
          : undefined
      }
      onDrop={
        dragHandlers
          ? (e) => {
              e.preventDefault();
              dragHandlers.onDrop();
            }
          : undefined
      }
      className={`group relative flex items-start gap-2 rounded-md px-1 py-1 ${
        highlighted ? 'bg-amber-500/10 ring-1 ring-amber-500' : ''
      } ${editing ? 'focus-within:bg-stitch-elevated/50' : ''}`}
    >
      {drop ? (
        <span
          aria-hidden
          className={`pointer-events-none absolute left-8 right-2 h-0.5 rounded-full bg-stitch-accent ${
            drop === 'before' ? '-top-px' : '-bottom-px'
          }`}
        />
      ) : null}
      {editing ? (
        <span
          draggable={Boolean(dragHandlers)}
          onDragStart={(e) => {
            e.dataTransfer.effectAllowed = 'move';
            e.dataTransfer.setData('text/plain', id);
            dragHandlers?.onDragStart();
          }}
          onDragEnd={() => dragHandlers?.onDragEnd()}
          title="Drag to reorder (or Alt+↑/↓)"
          className="material-symbols-outlined mt-0.5 cursor-grab text-base text-transparent group-hover:text-stitch-muted group-focus-within:text-stitch-muted"
        >
          drag_indicator
        </span>
      ) : null}
      {badge}
      {editing ? (
        <textarea
          ref={ref}
          value={value}
          rows={1}
          aria-label={variant === 'bullet' ? 'Condition' : `Step ${label ?? ''}`}
          onChange={(e) => setValue(e.target.value)}
          onBlur={commit}
          onKeyDown={onKeyDown}
          className="min-w-0 flex-1 resize-none overflow-hidden bg-transparent text-sm leading-6 text-stitch-fg outline-hidden placeholder:text-stitch-muted"
          placeholder={variant === 'bullet' ? 'Describe the condition…' : 'Describe the step…'}
        />
      ) : (
        <div className="min-w-0 flex-1 text-sm leading-6 text-stitch-fg">
          <StatementText source={text} />
        </div>
      )}
      {trailing}
      {editing && (onInsertBelow || menu) ? (
        <span className="mt-0.5 flex items-center gap-0.5 text-stitch-muted opacity-0 group-hover:opacity-100 group-focus-within:opacity-100">
          {onInsertBelow ? (
            <button
              type="button"
              title="Add below"
              aria-label="Add below"
              onClick={onInsertBelow}
              className="rounded-sm p-0.5 hover:bg-stitch-elevated"
            >
              <span className="material-symbols-outlined text-base">add</span>
            </button>
          ) : null}
          {menu ? (
            <span className="relative" ref={menuRef}>
              <button
                type="button"
                title="More"
                aria-label="More actions"
                aria-haspopup="menu"
                aria-expanded={menuOpen}
                onClick={() => setMenuOpen((o) => !o)}
                className="rounded-sm p-0.5 hover:bg-stitch-elevated"
              >
                <span className="material-symbols-outlined text-base">more_horiz</span>
              </button>
              {menuOpen ? (
                <div
                  role="menu"
                  className="absolute right-0 top-7 z-30 w-72 rounded-lg border border-stitch-border bg-stitch-surface py-1 text-sm shadow-stitch"
                >
                  {menu.map((item) => (
                    <button
                      key={item.label}
                      type="button"
                      role="menuitem"
                      disabled={item.disabled}
                      onClick={() => {
                        setMenuOpen(false);
                        item.onSelect();
                      }}
                      className={`flex w-full items-center gap-2.5 px-3 py-1.5 text-left hover:bg-stitch-elevated disabled:opacity-40 ${
                        item.danger ? 'text-stitch-danger' : 'text-stitch-fg'
                      }`}
                    >
                      <span
                        className={`material-symbols-outlined text-base ${item.danger ? '' : 'text-stitch-muted'}`}
                      >
                        {item.icon}
                      </span>
                      {item.label}
                    </button>
                  ))}
                </div>
              ) : null}
            </span>
          ) : null}
        </span>
      ) : null}
    </div>
  );
}
