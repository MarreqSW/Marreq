import type { ReactNode } from 'react';

/** Shared breadcrumb + title block (RVM / Stitch shell). */
export default function StitchPageHeader({
  projectName,
  section,
  title,
  subtitle,
  children,
}: {
  projectName: string;
  section: string;
  title: string;
  subtitle?: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between mb-8">
      <div>
        <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
          <span>{projectName}</span>
          <span className="mx-2">/</span>
          <span className="text-stitch-accent font-bold">{section}</span>
        </nav>
        <h2 className="text-2xl md:text-3xl font-extrabold text-white tracking-tight font-headline">
          {title}
        </h2>
        {subtitle ? (
          <p className="text-stitch-muted text-sm mt-2 max-w-2xl">{subtitle}</p>
        ) : null}
      </div>
      {children ? <div className="shrink-0 flex flex-wrap gap-2">{children}</div> : null}
    </div>
  );
}
