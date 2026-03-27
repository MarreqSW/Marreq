import { NavLink, useLocation, useParams } from 'react-router-dom';

export default function VerificationsViewSwitcher() {
  const { projectId } = useParams();
  const loc = useLocation();
  const pid = projectId ?? '';

  const sp = new URLSearchParams(loc.search);
  const listView = sp.get('view') === 'list';
  const onVerSection = /\/verifications\/?$/.test(loc.pathname);

  const tableActive = onVerSection && !listView;
  const listActive = onVerSection && listView;

  const seg =
    'flex items-center gap-2 px-4 py-1.5 text-xs font-bold rounded-md transition-colors';
  const inactive =
    'text-stitch-muted hover:bg-stitch-higher hover:text-stitch-fg';
  const active = 'bg-stitch-elevated text-stitch-accent shadow-stitch-inset border border-stitch-border';

  return (
    <div className="flex p-1 bg-stitch-surface rounded-lg gap-1 border border-stitch-border">
      <NavLink
        to={`/p/${pid}/verifications`}
        className={() => `${seg} ${tableActive ? active : inactive}`}
      >
        <span className="material-symbols-outlined text-sm">table_rows</span>
        Table
      </NavLink>
      <NavLink
        to={`/p/${pid}/verifications?view=list`}
        className={() => `${seg} ${listActive ? active : inactive}`}
      >
        <span className="material-symbols-outlined text-sm">view_list</span>
        List
      </NavLink>
    </div>
  );
}
