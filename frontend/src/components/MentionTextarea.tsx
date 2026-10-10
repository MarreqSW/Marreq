import { useId, useLayoutEffect, useRef, useState } from 'react';
import type { ProjectMember } from '@/api/types';
import { activeMention, suggestMembers } from '@/utils/mentions';

/**
 * A textarea that suggests project members after `@`. Arrow keys move through the
 * list, Enter or Tab inserts `@username `, and Escape closes it.
 */
export default function MentionTextarea({
  value,
  onChange,
  members,
  className,
  placeholder,
  ariaLabel,
}: {
  value: string;
  onChange: (value: string) => void;
  members: ProjectMember[];
  className?: string;
  placeholder?: string;
  ariaLabel: string;
}) {
  const listId = useId();
  const ref = useRef<HTMLTextAreaElement | null>(null);
  const [caret, setCaret] = useState(0);
  const [active, setActive] = useState(0);
  const [dismissedAt, setDismissedAt] = useState<number | null>(null);
  // Caret to restore once an inserted mention has been rendered.
  const pendingCaret = useRef<number | null>(null);

  useLayoutEffect(() => {
    const at = pendingCaret.current;
    if (at === null || !ref.current) return;
    pendingCaret.current = null;
    ref.current.focus();
    ref.current.setSelectionRange(at, at);
  }, [value]);

  const mention = members.length > 0 ? activeMention(value, caret) : null;
  const suggestions = mention ? suggestMembers(members, mention.query) : [];
  const open = mention !== null && suggestions.length > 0 && dismissedAt !== mention.start;
  const optionId = (i: number) => `${listId}-option-${i}`;

  const insert = (member: ProjectMember) => {
    if (!mention) return;
    const text = `@${member.username} `;
    const next = value.slice(0, mention.start) + text + value.slice(caret);
    const at = mention.start + text.length;
    pendingCaret.current = at;
    onChange(next);
    setCaret(at);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (!open) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive((i) => Math.min(i + 1, suggestions.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' || e.key === 'Tab') {
      e.preventDefault();
      insert(suggestions[Math.min(active, suggestions.length - 1)]!);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      setDismissedAt(mention.start);
    }
  };

  const track = (el: HTMLTextAreaElement) => {
    setCaret(el.selectionStart ?? el.value.length);
  };

  return (
    <div>
      <textarea
        ref={ref}
        className={className}
        placeholder={placeholder}
        value={value}
        aria-label={ariaLabel}
        role={members.length > 0 ? 'combobox' : undefined}
        aria-autocomplete={members.length > 0 ? 'list' : undefined}
        aria-expanded={members.length > 0 ? open : undefined}
        aria-controls={open ? listId : undefined}
        aria-activedescendant={open ? optionId(active) : undefined}
        onChange={(e) => {
          onChange(e.target.value);
          track(e.target);
          setActive(0);
          setDismissedAt(null);
        }}
        onKeyDown={onKeyDown}
        onKeyUp={(e) => track(e.currentTarget)}
        onClick={(e) => track(e.currentTarget)}
        onBlur={() => setDismissedAt(mention?.start ?? null)}
        onFocus={() => setDismissedAt(null)}
      />
      {/* In the page flow rather than floating: comment boxes sit in cards that clip overflow. */}
      {open ? (
        <ul
          id={listId}
          role="listbox"
          aria-label="Mention a project member"
          className="mt-1 max-h-64 overflow-y-auto rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch py-1"
        >
          {suggestions.map((m, i) => (
            <li
              key={m.user_id}
              id={optionId(i)}
              role="option"
              aria-selected={i === active}
              onMouseEnter={() => setActive(i)}
              onMouseDown={(e) => {
                e.preventDefault();
                insert(m);
              }}
              className={`flex items-baseline gap-2 px-3 py-1.5 text-sm cursor-pointer ${
                i === active ? 'bg-stitch-elevated' : ''
              }`}
            >
              <span className="font-semibold text-stitch-fg">{m.name}</span>
              <span className="text-xs text-stitch-muted">@{m.username}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
