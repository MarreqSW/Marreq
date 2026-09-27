import { useId, useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react';
import StatementText from '@/components/StatementText';
import {
  insertLink,
  toggleList,
  toggleWrap,
  utf8Length,
  type EditResult,
} from '@/utils/statementEditing';

export const STATEMENT_MAX_BYTES = 2000;

type StatementEditorProps = {
  id: string;
  value: string;
  onChange: (value: string) => void;
  required?: boolean;
  disabled?: boolean;
  placeholder?: string;
  /** Accessible name when there is no `<label htmlFor={id}>`. */
  ariaLabel?: string;
  /** Tailwind min-height class for the textarea and the preview. */
  minHeightClass?: string;
  textareaClassName?: string;
};

type Action = {
  label: string;
  icon: string;
  shortcut?: string;
  apply: (value: string, start: number, end: number) => EditResult;
};

const ACTIONS: Action[] = [
  { label: 'Bold', icon: 'format_bold', shortcut: 'b', apply: (v, s, e) => toggleWrap(v, s, e, '**') },
  { label: 'Italic', icon: 'format_italic', shortcut: 'i', apply: (v, s, e) => toggleWrap(v, s, e, '*') },
  { label: 'Code', icon: 'code', apply: (v, s, e) => toggleWrap(v, s, e, '`') },
  { label: 'Bulleted list', icon: 'format_list_bulleted', apply: (v, s, e) => toggleList(v, s, e, 'bullets') },
  { label: 'Numbered list', icon: 'format_list_numbered', apply: (v, s, e) => toggleList(v, s, e, 'numbered') },
  { label: 'Link', icon: 'link', shortcut: 'k', apply: insertLink },
];

const tabBase = 'px-3 py-1 text-xs font-bold uppercase tracking-wider rounded-md transition-colors';

/**
 * Requirement statement editor: a textarea with a formatting toolbar for the
 * Marreq statement Markdown subset, a Write / Preview toggle, and a byte counter.
 */
export default function StatementEditor({
  id,
  value,
  onChange,
  required,
  disabled,
  placeholder,
  ariaLabel,
  minHeightClass = 'min-h-[200px]',
  textareaClassName = '',
}: StatementEditorProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const pendingSelection = useRef<[number, number] | null>(null);
  const [preview, setPreview] = useState(false);
  const helpId = useId();

  useLayoutEffect(() => {
    const sel = pendingSelection.current;
    const el = textareaRef.current;
    if (sel && el) {
      el.focus();
      el.setSelectionRange(sel[0], sel[1]);
      pendingSelection.current = null;
    }
  });

  const run = (action: Action) => {
    const el = textareaRef.current;
    if (!el || disabled) return;
    const result = action.apply(value, el.selectionStart, el.selectionEnd);
    pendingSelection.current = [result.selectionStart, result.selectionEnd];
    onChange(result.value);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return;
    const action = ACTIONS.find((a) => a.shortcut === e.key.toLowerCase());
    if (action) {
      e.preventDefault();
      run(action);
    }
  };

  const bytes = utf8Length(value);
  const over = bytes > STATEMENT_MAX_BYTES;

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div role="tablist" aria-label="Statement editor mode" className="flex gap-1">
          <button
            type="button"
            role="tab"
            aria-selected={!preview}
            onClick={() => setPreview(false)}
            className={`${tabBase} ${!preview ? 'bg-stitch-elevated text-stitch-accent' : 'text-stitch-muted hover:text-stitch-fg'}`}
          >
            Write
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={preview}
            onClick={() => setPreview(true)}
            className={`${tabBase} ${preview ? 'bg-stitch-elevated text-stitch-accent' : 'text-stitch-muted hover:text-stitch-fg'}`}
          >
            Preview
          </button>
        </div>
        <div role="toolbar" aria-label="Formatting" className="flex gap-1">
          {ACTIONS.map((a) => (
            <button
              key={a.label}
              type="button"
              aria-label={a.label}
              title={a.shortcut ? `${a.label} (Ctrl+${a.shortcut.toUpperCase()})` : a.label}
              disabled={preview || disabled}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => run(a)}
              className="p-1 rounded-sm text-stitch-muted hover:bg-stitch-higher hover:text-stitch-fg disabled:opacity-40 disabled:hover:bg-transparent"
            >
              <span className="material-symbols-outlined text-lg" aria-hidden="true">
                {a.icon}
              </span>
            </button>
          ))}
        </div>
      </div>

      {preview ? (
        <div
          className={`w-full rounded-lg border border-stitch-border bg-stitch-surface p-4 text-sm leading-relaxed text-stitch-fg ${minHeightClass}`}
          data-testid="statement-preview"
        >
          <StatementText source={value} empty={<span className="text-stitch-muted">Nothing to preview.</span>} />
        </div>
      ) : null}
      <textarea
        id={id}
        ref={textareaRef}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        // A hidden required field would block submit without a visible message.
        required={required && !preview}
        disabled={disabled}
        placeholder={placeholder}
        aria-label={ariaLabel}
        aria-describedby={helpId}
        hidden={preview}
        className={`${textareaClassName} ${minHeightClass}`}
      />

      <div id={helpId} className="flex flex-wrap items-start justify-between gap-2 text-xs text-stitch-muted">
        <details>
          <summary className="cursor-pointer select-none">Formatting help</summary>
          <p className="mt-1 font-mono leading-relaxed">
            **bold** · *italic* · `code` · [label](https://…)
            <br />- bulleted item · 1. numbered item · blank line = new paragraph
          </p>
        </details>
        <span className={over ? 'font-semibold text-red-700 dark:text-red-300' : ''} aria-live="polite">
          {bytes} / {STATEMENT_MAX_BYTES} bytes
        </span>
      </div>
    </div>
  );
}
