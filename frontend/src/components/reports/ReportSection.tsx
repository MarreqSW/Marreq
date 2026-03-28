import type { ReactNode } from 'react';

export function ReportSection({
  id,
  title,
  subtitle,
  children,
}: {
  id?: string;
  title: string;
  subtitle?: string;
  children: ReactNode;
}) {
  return (
    <section
      id={id}
      className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch scroll-mt-24"
    >
      <div className="px-4 py-3 border-b border-stitch-border bg-stitch-elevated">
        <h3 className="text-sm font-bold text-stitch-fg">{title}</h3>
        {subtitle ? (
          <p className="text-[10px] text-stitch-muted mt-1 uppercase tracking-wide">{subtitle}</p>
        ) : null}
      </div>
      {children}
    </section>
  );
}
