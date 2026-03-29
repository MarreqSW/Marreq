import { useEffect, useState } from 'react';
import {
  Navigate,
  Outlet,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from 'react-router-dom';
import { DashboardProvider, useDashboard } from '@/context/DashboardContext';
import HomeRedirect from '@/pages/HomeRedirect';
import LoginPage from '@/pages/LoginPage';
import ProjectLayout from '@/pages/ProjectLayout';
import AdminPage from '@/pages/AdminPage';
import CreateRequirementPage from '@/pages/CreateRequirementPage';
import CreateVerificationPage from '@/pages/CreateVerificationPage';
import DashboardPage from '@/pages/DashboardPage';
import EditRequirementPage from '@/pages/EditRequirementPage';
import EditVerificationPage from '@/pages/EditVerificationPage';
import ViewRequirementPage from '@/pages/ViewRequirementPage';
import ViewVerificationPage from '@/pages/ViewVerificationPage';
import HelpPage from '@/pages/HelpPage';
import ProjectSettingsPage from '@/pages/ProjectSettingsPage';
import ReportsPage from '@/pages/ReportsPage';
import RequirementsPage from '@/pages/RequirementsPage';
import TraceabilityPage from '@/pages/TraceabilityPage';
import VerificationsPage from '@/pages/VerificationsPage';
import MatrixPage from '@/pages/MatrixPage';
import BaselinesPage from '@/pages/BaselinesPage';
import BaselineDetailPage from '@/pages/BaselineDetailPage';
import ProjectCatalogLayout from '@/pages/catalog/ProjectCatalogLayout';
import CatalogCategoriesPage from '@/pages/catalog/CatalogCategoriesPage';
import CatalogApplicabilityPage from '@/pages/catalog/CatalogApplicabilityPage';
import CatalogRequirementStatusesPage from '@/pages/catalog/CatalogRequirementStatusesPage';
import CatalogVerificationStatusesPage from '@/pages/catalog/CatalogVerificationStatusesPage';
import CatalogCustomFieldsPage from '@/pages/catalog/CatalogCustomFieldsPage';
import CatalogVerificationMethodsPage from '@/pages/catalog/CatalogVerificationMethodsPage';
import GroupsListPage from '@/pages/groups/GroupsListPage';
import GroupCreatePage from '@/pages/groups/GroupCreatePage';
import GroupViewPage from '@/pages/groups/GroupViewPage';
import GroupEditPage from '@/pages/groups/GroupEditPage';
import GroupMembersPage from '@/pages/groups/GroupMembersPage';

function ProtectedShell() {
  const { refresh, loading, dashboard, error } = useDashboard();
  const navigate = useNavigate();
  const location = useLocation();
  const [bootError, setBootError] = useState<string | null>(null);

  useEffect(() => {
    refresh().catch(() => {
      setBootError('unauthorized');
      navigate('/login', { replace: true, state: { from: location.pathname } });
    });
  }, [refresh, navigate, location.pathname]);

  if (bootError || (!loading && !dashboard)) {
    return null;
  }

  if (loading && !dashboard) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-stitch-canvas text-stitch-muted text-sm">
        Loading…
      </div>
    );
  }

  if (error && !dashboard) {
    return null;
  }

  return <Outlet />;
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route
        element={
          <DashboardProvider>
            <ProtectedShell />
          </DashboardProvider>
        }
      >
        <Route index element={<HomeRedirect />} />
        {/* Groups routes (reserved namespace — matched before :namespace catch-all) */}
        <Route path="groups" element={<GroupsListPage />} />
        <Route path="groups/new" element={<GroupCreatePage />} />
        <Route path="groups/:groupId" element={<GroupViewPage />} />
        <Route path="groups/:groupId/edit" element={<GroupEditPage />} />
        <Route path="groups/:groupId/members" element={<GroupMembersPage />} />
        {/* Namespace-scoped project routes: /:namespace/:projectSlug */}
        <Route path=":namespace/:projectSlug" element={<ProjectLayout />}>
          <Route index element={<Navigate to="dashboard" replace />} />
          <Route path="dashboard" element={<DashboardPage />} />
          <Route path="requirements/new" element={<CreateRequirementPage />} />
          <Route path="requirements/:requirementId/edit" element={<EditRequirementPage />} />
          <Route path="requirements/:requirementId" element={<ViewRequirementPage />} />
          <Route path="requirements" element={<RequirementsPage />} />
          <Route path="verifications/new" element={<CreateVerificationPage />} />
          <Route path="verifications/:verificationId/edit" element={<EditVerificationPage />} />
          <Route path="verifications/:verificationId" element={<ViewVerificationPage />} />
          <Route path="verifications" element={<VerificationsPage />} />
          <Route path="traceability" element={<TraceabilityPage />} />
          <Route path="matrix" element={<MatrixPage />} />
          <Route path="baselines/:baselineId" element={<BaselineDetailPage />} />
          <Route path="baselines" element={<BaselinesPage />} />
          <Route path="reports" element={<ReportsPage />} />
          <Route path="settings" element={<ProjectSettingsPage />} />
          {/* Old / classic URL; avoid full-page navigation to Rocket (404 on :8000). */}
          <Route path="members" element={<Navigate to="settings" replace />} />
          <Route path="catalog" element={<ProjectCatalogLayout />}>
            <Route index element={<Navigate to="categories" replace />} />
            <Route path="categories" element={<CatalogCategoriesPage />} />
            <Route path="applicability" element={<CatalogApplicabilityPage />} />
            <Route path="requirement-statuses" element={<CatalogRequirementStatusesPage />} />
            <Route path="verification-statuses" element={<CatalogVerificationStatusesPage />} />
            <Route path="custom-fields" element={<CatalogCustomFieldsPage />} />
            <Route path="verification-methods" element={<CatalogVerificationMethodsPage />} />
          </Route>
          <Route path="help" element={<HelpPage />} />
          <Route path="admin" element={<AdminPage />} />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
