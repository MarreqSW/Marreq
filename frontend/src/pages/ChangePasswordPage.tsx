import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { changePassword, getCsrfToken } from '@/api/client';
import AuthLayout from '@/components/AuthLayout';
import { useDashboard } from '@/context/DashboardContext';
import { useFormSubmit } from '@/hooks/useFormSubmit';

export default function ChangePasswordPage() {
  const navigate = useNavigate();
  const { csrfToken } = useDashboard();
  const [currentPassword, setCurrentPassword] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  const { error, submitting, onSubmit } = useFormSubmit(async () => {
    if (password !== confirmPassword) {
      throw new Error('Passwords do not match');
    }
    const token = csrfToken ?? (await getCsrfToken());
    await changePassword(
      {
        current_password: currentPassword,
        new_password: password,
        confirm_password: confirmPassword,
      },
      token,
    );
    navigate('/login', { replace: true });
  });

  return (
    <AuthLayout
      title="Change password"
      subtitle="Enter your current password and choose a new one. You will need to sign in again."
      footer={
        <p className="text-center text-sm">
          <Link to="/" className="text-stitch-accent hover:underline">
            Back to home
          </Link>
        </p>
      }
    >
      <form onSubmit={onSubmit} className="space-y-4">
        {error && (
          <div className="rounded-lg bg-red-500/10 border border-red-500/25 px-3 py-2 text-sm text-red-800 dark:text-red-200">
            {error}
          </div>
        )}
        <div>
          <label
            htmlFor="current-password"
            className="block text-xs font-semibold text-stitch-muted uppercase mb-1"
          >
            Current password
          </label>
          <input
            id="current-password"
            type="password"
            autoComplete="current-password"
            className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            required
          />
        </div>
        <div>
          <label
            htmlFor="new-password"
            className="block text-xs font-semibold text-stitch-muted uppercase mb-1"
          >
            New password
          </label>
          <input
            id="new-password"
            type="password"
            autoComplete="new-password"
            className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </div>
        <div>
          <label
            htmlFor="confirm-password"
            className="block text-xs font-semibold text-stitch-muted uppercase mb-1"
          >
            Confirm new password
          </label>
          <input
            id="confirm-password"
            type="password"
            autoComplete="new-password"
            className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
          />
        </div>
        <button
          type="submit"
          disabled={submitting}
          className="w-full rounded-lg bg-gradient-to-br from-[#000666] to-[#1a237e] text-white font-semibold py-2.5 text-sm hover:opacity-95 disabled:opacity-60"
        >
          {submitting ? 'Updating…' : 'Update password'}
        </button>
      </form>
    </AuthLayout>
  );
}
