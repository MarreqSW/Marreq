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
import StartPage from '@/pages/StartPage';
import LoginPage from '@/pages/LoginPage';
import RegisterPage from '@/pages/RegisterPage';
import ForgotPasswordPage from '@/pages/ForgotPasswordPage';
import ResetPasswordPage from '@/pages/ResetPasswordPage';
import ChangePasswordPage from '@/pages/ChangePasswordPage';
import AccountPage from '@/pages/AccountPage';
import VerifyEmailPage from '@/pages/VerifyEmailPage';
import ProjectLayout from '@/pages/ProjectLayout';
import LegacyNamespaceProjectRedirect from '@/pages/LegacyNamespaceProjectRedirect';
import ClassicRequirementShowRedirect from '@/pages/ClassicRequirementShowRedirect';
import AdminPage from '@/pages/AdminPage';
import SystemLogsPage from '@/pages/SystemLogsPage';
import LogAnalyticsPage from '@/pages/LogAnalyticsPage';
import BackupPage from '@/pages/BackupPage';
import CreateRequirementPage from '@/pages/CreateRequirementPage';
import CreateVerificationPage from '@/pages/CreateVerificationPage';
import DashboardPage from '@/pages/DashboardPage';
import EditRequirementPage from '@/pages/EditRequirementPage';
import EditVerificationPage from '@/pages/EditVerificationPage';
import ViewRequirementPage from '@/pages/ViewRequirementPage';
import ViewVerificationPage from '@/pages/ViewVerificationPage';
import HelpPage from '@/pages/HelpPage';
import ImportPage from '@/pages/ImportPage';
import ProjectSettingsLayout from '@/pages/settings/ProjectSettingsLayout';
import GeneralSettingsPage from '@/pages/settings/GeneralSettingsPage';
import MembersSettingsPage from '@/pages/settings/MembersSettingsPage';
import NotificationSettingsPage from '@/pages/settings/NotificationSettingsPage';
import StorageSettingsPage from '@/pages/settings/StorageSettingsPage';
import ProjectCreatePage from '@/pages/ProjectCreatePage';
import ProjectBundleImportPage from '@/pages/ProjectBundleImportPage';
import ReportBuilderPage from '@/pages/ReportBuilderPage';
import ReportsPage from '@/pages/ReportsPage';
import RequirementsPage from '@/pages/RequirementsPage';
import TraceabilityPage from '@/pages/TraceabilityPage';
import VerificationsPage from '@/pages/VerificationsPage';
import { MatrixRedirect, MovedRedirect } from '@/pages/LegacyRedirects';
import AdminLayout from '@/pages/admin/AdminLayout';
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
import CompatibilityBanner from '@/components/CompatibilityBanner';

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

  return (
    <>
      <CompatibilityBanner />
      <Outlet />
    </>
  );
}

export default function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/verify-email" element={<VerifyEmailPage />} />
      <Route path="/forgot-password" element={<ForgotPasswordPage />} />
      <Route path="/reset-password" element={<ResetPasswordPage />} />
      <Route
        element={
          <DashboardProvider>
            <ProtectedShell />
          </DashboardProvider>
        }
      >
        <Route index element={<StartPage />} />
        <Route path="change-password" element={<ChangePasswordPage />} />
        <Route path="account" element={<AccountPage />} />
        <Route path="change_password" element={<Navigate to="/change-password" replace />} />
        <Route path="projects/new" element={<ProjectCreatePage />} />
        <Route path="projects/import-bundle" element={<ProjectBundleImportPage />} />
        {/* Groups routes (reserved — matched before :projectSlug catch-all) */}
        <Route path="groups" element={<GroupsListPage />} />
        <Route path="groups/new" element={<GroupCreatePage />} />
        <Route path="groups/:groupId" element={<GroupViewPage />} />
        <Route path="groups/:groupId/edit" element={<GroupEditPage />} />
        <Route path="groups/:groupId/members" element={<GroupMembersPage />} />
        {/* Instance administration ("admin" is a reserved namespace, so no project can shadow it). */}
        <Route path="admin" element={<AdminLayout />}>
          <Route index element={<AdminPage />} />
          <Route path="logs" element={<SystemLogsPage />} />
          <Route path="logs/analytics" element={<LogAnalyticsPage />} />
          <Route path="backup" element={<BackupPage />} />
        </Route>
        {/* Project workspace: /:projectSlug/... */}
        <Route path=":projectSlug" element={<ProjectLayout />}>
          <Route index element={<Navigate to="dashboard" replace />} />
          <Route path="dashboard" element={<DashboardPage />} />
          <Route path="requirements/new" element={<CreateRequirementPage />} />
          <Route path="requirements/:requirementId/edit" element={<EditRequirementPage />} />
          <Route
            path="requirements/:requirementId/versions/:versionId"
            element={<ViewRequirementPage />}
          />
          <Route
            path="requirements/show/:requirementId/version/:versionId"
            element={<ClassicRequirementShowRedirect />}
          />
          <Route
            path="requirements/show/:requirementId"
            element={<ClassicRequirementShowRedirect />}
          />
          <Route path="requirements/:requirementId" element={<ViewRequirementPage />} />
          <Route path="requirements" element={<RequirementsPage />} />
          <Route path="verifications/new" element={<CreateVerificationPage />} />
          <Route path="verifications/:verificationId/edit" element={<EditVerificationPage />} />
          <Route path="verifications/:verificationId" element={<ViewVerificationPage />} />
          <Route path="verifications" element={<VerificationsPage />} />
          <Route path="traceability" element={<TraceabilityPage />} />
          <Route path="matrix" element={<MatrixRedirect />} />
          <Route path="baselines/:baselineId" element={<BaselineDetailPage />} />
          <Route path="baselines" element={<BaselinesPage />} />
          <Route path="reports/builder" element={<ReportBuilderPage />} />
          <Route path="reports" element={<ReportsPage />} />
          <Route path="settings" element={<ProjectSettingsLayout />}>
            <Route index element={<Navigate to="general" replace />} />
            <Route path="general" element={<GeneralSettingsPage />} />
            <Route path="members" element={<MembersSettingsPage />} />
            <Route path="catalog" element={<ProjectCatalogLayout />}>
              <Route index element={<Navigate to="categories" replace />} />
              <Route path="categories" element={<CatalogCategoriesPage />} />
              <Route path="applicability" element={<CatalogApplicabilityPage />} />
              <Route path="requirement-statuses" element={<CatalogRequirementStatusesPage />} />
              <Route path="verification-statuses" element={<CatalogVerificationStatusesPage />} />
              <Route path="custom-fields" element={<CatalogCustomFieldsPage />} />
              <Route path="verification-methods" element={<CatalogVerificationMethodsPage />} />
            </Route>
            <Route path="storage" element={<StorageSettingsPage />} />
            <Route path="notifications" element={<NotificationSettingsPage />} />
            <Route path="import" element={<ImportPage embedded />} />
          </Route>
          {/* Pages that moved into Project settings (issue #346). */}
          <Route path="import" element={<MovedRedirect to="settings/import" />} />
          <Route path="members" element={<MovedRedirect to="settings/members" />} />
          <Route path="catalog/*" element={<MovedRedirect to="settings/catalog" />} />
          <Route path="help" element={<HelpPage />} />
          {/* Instance administration moved to /admin (issue #346). */}
          <Route path="admin/*" element={<MovedRedirect to="/admin" />} />
        </Route>
        <Route
          path=":namespace/:projectSlug/*"
          element={<LegacyNamespaceProjectRedirect />}
        />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
