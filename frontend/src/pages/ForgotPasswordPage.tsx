import { useState } from 'react';
import { Link } from 'react-router-dom';
import { requestPasswordReset } from '@/api/client';
import AuthLayout from '@/components/AuthLayout';
import { useFormSubmit } from '@/hooks/useFormSubmit';

export default function ForgotPasswordPage() {
  const [email, setEmail] = useState('');
  const [submitted, setSubmitted] = useState(false);

  const { error, submitting, onSubmit } = useFormSubmit(async () => {
    await requestPasswordReset({ email });
    setSubmitted(true);
  });

  return (
    <AuthLayout
      title="Reset your password"
      subtitle="Enter your email address and Marreq will send reset instructions when possible."
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
          <p className="font-semibold">Check your email</p>
          <p className="mt-2 text-stitch-muted">
            If that email is registered, a password-reset link has been sent.
          </p>
        </div>
      ) : (
        <form onSubmit={onSubmit} className="space-y-4">
          {error && (
            <div className="rounded-lg bg-red-500/10 border border-red-500/25 px-3 py-2 text-sm text-red-800 dark:text-red-200">
              {error}
            </div>
          )}
          <div>
            <label htmlFor="email" className="block text-xs font-semibold text-stitch-muted uppercase mb-1">
              Email
            </label>
            <input
              id="email"
              type="email"
              autoComplete="email"
              className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
            />
          </div>
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded-lg bg-gradient-to-br from-[#000666] to-[#1a237e] text-white font-semibold py-2.5 text-sm hover:opacity-95 disabled:opacity-60"
          >
            {submitting ? 'Sending…' : 'Send reset link'}
          </button>
        </form>
      )}
    </AuthLayout>
  );
}
