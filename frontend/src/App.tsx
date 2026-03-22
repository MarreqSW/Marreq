import { useEffect, useState } from 'react';
import { Navigate, Outlet, Route, Routes, useLocation, useNavigate } from 'react-router-dom';
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
import HelpPage from '@/pages/HelpPage';
import ProjectSettingsPage from '@/pages/ProjectSettingsPage';
import ReportsPage from '@/pages/ReportsPage';
import RequirementsPage from '@/pages/RequirementsPage';
import TraceabilityPage from '@/pages/TraceabilityPage';
import VerificationsPage from '@/pages/VerificationsPage';
import MatrixPage from '@/pages/MatrixPage';
import BaselinesPage from '@/pages/BaselinesPage';
import BaselineDetailPage from '@/pages/BaselineDetailPage';

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
      <div className="min-h-screen flex items-center justify-center text-slate-500 text-sm">
        Loading…
      </div>
    );
  }

  if (error && !dashboard) {
    return null;
  }

  if (dashboard && (dashboard.projects?.length ?? 0) === 0) {
    return (
      <div className="min-h-screen flex flex-col items-center justify-center p-6 text-slate-600">
        <p>No projects available for your account.</p>
      </div>
    );
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
        <Route path="p/:projectId" element={<ProjectLayout />}>
          <Route index element={<Navigate to="dashboard" replace />} />
          <Route path="dashboard" element={<DashboardPage />} />
          <Route path="requirements/new" element={<CreateRequirementPage />} />
          <Route path="requirements" element={<RequirementsPage />} />
          <Route path="requirements/:requirementId/edit" element={<EditRequirementPage />} />
          <Route path="verifications/new" element={<CreateVerificationPage />} />
          <Route path="verifications/:verificationId/edit" element={<EditVerificationPage />} />
          <Route path="verifications" element={<VerificationsPage />} />
          <Route path="traceability" element={<TraceabilityPage />} />
          <Route path="matrix" element={<MatrixPage />} />
          <Route path="baselines/:baselineId" element={<BaselineDetailPage />} />
          <Route path="baselines" element={<BaselinesPage />} />
          <Route path="reports" element={<ReportsPage />} />
          <Route path="settings" element={<ProjectSettingsPage />} />
          <Route path="help" element={<HelpPage />} />
          <Route path="admin" element={<AdminPage />} />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
