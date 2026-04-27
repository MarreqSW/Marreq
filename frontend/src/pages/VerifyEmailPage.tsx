import { useEffect, useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { verifyEmail } from '@/api/client';
import AuthLayout from '@/components/AuthLayout';

export default function VerifyEmailPage() {
  const [searchParams] = useSearchParams();
  const token = useMemo(() => searchParams.get('token') ?? '', [searchParams]);
  const [status, setStatus] = useState<'loading' | 'success' | 'error'>(
    token ? 'loading' : 'error',
  );
  const [message, setMessage] = useState(token ? 'Verifying your email…' : 'Missing verification token.');

  useEffect(() => {
    if (!token) return;
    let alive = true;
    verifyEmail(token)
      .then(() => {
        if (!alive) return;
        setStatus('success');
        setMessage('Your email address has been verified. You can now sign in.');
      })
      .catch((err) => {
        if (!alive) return;
        setStatus('error');
        setMessage(err instanceof Error ? err.message : 'Email verification failed');
      });
    return () => {
      alive = false;
    };
  }, [token]);

  return (
    <AuthLayout
      title="Verify your email"
      subtitle="Marreq Cloud verifies email ownership before allowing sign-in."
      footer={
        <p className="text-center text-sm">
          <Link to="/login" className="text-stitch-accent hover:underline">
            Back to sign in
          </Link>
        </p>
      }
    >
      <div
        className={`rounded-lg border p-4 text-sm ${
          status === 'success'
            ? 'border-emerald-500/30 bg-emerald-500/10 text-stitch-fg'
            : status === 'error'
              ? 'border-red-500/25 bg-red-500/10 text-red-800 dark:text-red-200'
              : 'border-stitch-border bg-stitch-elevated text-stitch-muted'
        }`}
      >
        {message}
      </div>
    </AuthLayout>
  );
}
