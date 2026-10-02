import { NavLink } from 'react-router-dom';

export type SubNavItem = {
  /** Route path, relative to the current route unless it starts with `/`. */
  to: string;
  label: string;
  icon?: string;
  /** Match only this exact path (no descendants). */
  end?: boolean;
  hidden?: boolean;
};

type Props = {
  items: SubNavItem[];
  ariaLabel: string;
  /** `secondary` is the smaller style for a second level (e.g. Catalog inside settings). */
  variant?: 'primary' | 'secondary';
};

/** Route-based tabs used by the settings hub, the catalog and the admin area. */
export default function SubNav({ items, ariaLabel, variant = 'primary' }: Props) {
  const primary = variant === 'primary';
  return (
    <nav
      aria-label={ariaLabel}
      className={`flex flex-wrap gap-2 border-b border-stitch-border ${primary ? 'mb-8 pb-4' : 'mb-6 pb-3'}`}
    >
      {items
        .filter((item) => !item.hidden)
        .map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.end}
            className={({ isActive }) =>
              `inline-flex items-center gap-1.5 rounded-md font-bold uppercase tracking-wide transition-colors ${
                primary ? 'px-3 py-2 text-xs' : 'px-2.5 py-1.5 text-[11px]'
              } ${
                isActive
                  ? primary
                    ? 'bg-stitch-accent text-stitch-canvas'
                    : 'bg-stitch-elevated text-stitch-accent'
                  : 'text-stitch-muted hover:text-stitch-fg hover:bg-stitch-higher'
              }`
            }
          >
            {item.icon ? (
              <span className="material-symbols-outlined text-base" aria-hidden>
                {item.icon}
              </span>
            ) : null}
            {item.label}
          </NavLink>
        ))}
    </nav>
  );
}
