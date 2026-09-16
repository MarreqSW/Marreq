import { Navigate, useLocation, useParams } from 'react-router-dom';

/**
 * Redirects bookmarked `/{owner-or-group}/{slug}/...` URLs to `/{slug}/...`.
 */
export default function LegacyNamespaceProjectRedirect() {
  const { projectSlug } = useParams();
  const location = useLocation();
  const rest = location.pathname.split('/').filter(Boolean).slice(2).join('/');
  if (!projectSlug) {
    return <Navigate to="/" replace />;
  }
  const to = `/${projectSlug}${rest ? `/${rest}` : ''}${location.search}${location.hash}`;
  return <Navigate to={to} replace />;
}
