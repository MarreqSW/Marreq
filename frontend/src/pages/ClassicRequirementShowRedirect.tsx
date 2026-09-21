import { Navigate, useParams } from 'react-router-dom';

/**
 * Redirects classic `/{slug}/requirements/show/:id` and
 * `/{slug}/requirements/show/:id/version/:versionId` bookmarks to the SPA.
 */
export default function ClassicRequirementShowRedirect() {
  const { projectSlug, requirementId, versionId } = useParams();
  if (!projectSlug || !requirementId) {
    return <Navigate to="/" replace />;
  }
  const to = versionId
    ? `/${projectSlug}/requirements/${requirementId}/versions/${versionId}`
    : `/${projectSlug}/requirements/${requirementId}`;
  return <Navigate to={to} replace />;
}
