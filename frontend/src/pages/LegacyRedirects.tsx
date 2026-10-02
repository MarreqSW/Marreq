import { Navigate, useLocation, useParams } from 'react-router-dom';

/**
 * Redirects for URLs that moved in the sidebar reorganization (issue #346).
 * They keep the query string, and for `*` routes the rest of the path.
 */

/** `/<project>/matrix?mx_…` → `/<project>/traceability?view=matrix&mx_…` */
export function MatrixRedirect() {
  const { projectSlug } = useParams();
  const params = new URLSearchParams(useLocation().search);
  params.set('view', 'matrix');
  return <Navigate to={`/${projectSlug}/traceability?${params.toString()}`} replace />;
}

/**
 * `/<project>/<old>/<rest>` → `<target>/<rest>`. `target` is relative to the
 * project (e.g. `settings/catalog`) or absolute (e.g. `/admin`).
 */
export function MovedRedirect({ to }: { to: string }) {
  const { projectSlug, '*': rest } = useParams();
  const { search, hash } = useLocation();
  const base = to.startsWith('/') ? to : `/${projectSlug}/${to}`;
  const tail = rest ? `/${rest}` : '';
  return <Navigate to={`${base}${tail}${search}${hash}`} replace />;
}
