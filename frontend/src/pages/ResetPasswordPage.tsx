import { FormEvent, useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { resetPassword } from '@/api/client';
import AuthLayout from '@/components/AuthLayout';

export default function ResetPasswordPage() {
  const [searchParams] = useSearchParams();
  const token = useMemo(() => searchParams.get('token') ?? '', [searchParams]);
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(token ? null : 'Missing reset token.');
  const [submitted, setSubmitted] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (password !== confirmPassword) {
      setError('Passwords do not match');
      return;
    }
    setError(null);
    setSubmitting(true);
    try {
      await resetPassword({ token, new_password: password });
      setSubmitted(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Password reset failed');
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <AuthLayout
      title="Choose a new password"
      subtitle="Use the reset link from your email to set a new password."
      footer={
        <p className="text-center text-sm">
          <Link to="/login" className="text-stitch-accent hover:underline">
            Back to sign in
          </Link>
        </p>
      }
    >
      {submitted ? (
        <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-4 text-sm text-stitch-fg">
          <p className="font-semibold">Password updated</p>
          <p className="mt-2 text-stitch-muted">You can now sign in with your new password.</p>
        </div>
      ) : (
        <form onSubmit={onSubmit} className="space-y-4">
          {error && (
            <div className="rounded-lg bg-red-500/10 border border-red-500/25 px-3 py-2 text-sm text-red-800 dark:text-red-200">
              {error}
            </div>
          )}
          <div>
            <label htmlFor="password" className="block text-xs font-semibold text-stitch-muted uppercase mb-1">
              New password
            </label>
            <input
              id="password"
              type="password"
              autoComplete="new-password"
              className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              disabled={!token}
            />
          </div>
          <div>
            <label htmlFor="confirm-password" className="block text-xs font-semibold text-stitch-muted uppercase mb-1">
              Confirm password
            </label>
            <input
              id="confirm-password"
              type="password"
              autoComplete="new-password"
              className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
              disabled={!token}
            />
          </div>
          <button
            type="submit"
            disabled={submitting || !token}
            className="w-full rounded-lg bg-gradient-to-br from-[#000666] to-[#1a237e] text-white font-semibold py-2.5 text-sm hover:opacity-95 disabled:opacity-60"
          >
            {submitting ? 'Updating…' : 'Update password'}
          </button>
        </form>
      )}
    </AuthLayout>
  );
}
