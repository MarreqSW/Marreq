import { Fragment, useMemo, type ReactNode } from 'react';
import { parseStatement, type Inline } from '@/utils/statementMarkdown';

function renderInline(nodes: Inline[], keyPrefix: string): ReactNode[] {
  return nodes.map((n, i) => {
    const key = `${keyPrefix}-${i}`;
    switch (n.kind) {
      case 'text':
        return <Fragment key={key}>{n.text}</Fragment>;
      case 'br':
        return <br key={key} />;
      case 'code':
        return (
          <code
            key={key}
            className="rounded bg-stitch-elevated px-1 py-0.5 font-mono text-[0.9em] text-stitch-fg"
          >
            {n.text}
          </code>
        );
      case 'strong':
        return (
          <strong key={key} className="font-semibold">
            {renderInline(n.children, key)}
          </strong>
        );
      case 'em':
        return <em key={key}>{renderInline(n.children, key)}</em>;
      case 'link':
        // `href` is restricted to http/https/mailto by the parser.
        return (
          <a
            key={key}
            href={n.href}
            target="_blank"
            rel="noopener noreferrer"
            className="text-stitch-accent underline underline-offset-2 hover:opacity-80"
          >
            {renderInline(n.children, key)}
          </a>
        );
    }
  });
}

type StatementTextProps = {
  /** Statement Markdown source. */
  source: string;
  /** Shown when the statement is empty. */
  empty?: ReactNode;
  className?: string;
};

/**
 * Renders a requirement statement (Marreq statement Markdown) as React elements.
 * No HTML strings are produced, so stored text can never inject markup.
 */
export default function StatementText({ source, empty = '—', className = '' }: StatementTextProps) {
  const blocks = useMemo(() => parseStatement(source), [source]);
  if (blocks.length === 0) {
    return <div className={className}>{empty}</div>;
  }
  return (
    <div className={`space-y-2 ${className}`} data-testid="statement-text">
      {blocks.map((b, i) => {
        const key = `b${i}`;
        if (b.kind === 'paragraph') return <p key={key}>{renderInline(b.inlines, key)}</p>;
        const items = b.items.map((it, k) => <li key={`${key}-${k}`}>{renderInline(it, `${key}-${k}`)}</li>);
        return b.kind === 'bullets' ? (
          <ul key={key} className="list-disc pl-6 space-y-0.5">
            {items}
          </ul>
        ) : (
          <ol key={key} start={b.start} className="list-decimal pl-6 space-y-0.5">
            {items}
          </ol>
        );
      })}
    </div>
  );
}
