import { useEffect, useState } from 'react';
import { setUserPassword } from '@/api/client';
import type { User } from '@/api/types';
import Dialog from '@/components/Dialog';
import { useFormSubmit } from '@/hooks/useFormSubmit';
import { btnPrimary, inp } from '@/pages/catalog/catalogUi';

type SetPasswordDialogProps = {
  user: User | null;
  getCsrfToken: () => Promise<string>;
  onClose: () => void;
  onSaved: (user: User) => void;
};

const label = 'block text-xs font-semibold text-stitch-fg mb-1';

export default function SetPasswordDialog({
  user,
  getCsrfToken,
  onClose,
  onSaved,
}: SetPasswordDialogProps) {
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  const { error, setError, submitting, onSubmit } = useFormSubmit(async () => {
    if (!user) return;
    await setUserPassword(user.id, password, confirmPassword, await getCsrfToken());
    onSaved(user);
  });

  useEffect(() => {
    setPassword('');
    setConfirmPassword('');
    setError(null);
  }, [user, setError]);

  return (
    <Dialog
      open={user !== null}
      onClose={onClose}
      title={user ? `Set password for ${user.username}` : 'Set password'}
      subtitle="The user is signed out of their other sessions."
    >
      <form onSubmit={onSubmit} className="space-y-4" data-testid="set-password-form">
        {error ? (
          <div
            role="alert"
            className="rounded-lg border border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100 text-sm px-4 py-2"
          >
            {error}
          </div>
        ) : null}
        <div>
          <label className={label} htmlFor="set-password-new">
            New password
          </label>
          <input
            id="set-password-new"
            type="password"
            className={inp}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            autoFocus
            autoComplete="new-password"
          />
        </div>
        <div>
          <label className={label} htmlFor="set-password-confirm">
            Confirm new password
          </label>
          <input
            id="set-password-confirm"
            type="password"
            className={inp}
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            required
            autoComplete="new-password"
          />
        </div>
        <div className="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={onClose}
            className="text-xs font-bold uppercase tracking-wider text-stitch-muted px-4 py-2 hover:text-stitch-fg"
          >
            Cancel
          </button>
          <button type="submit" className={btnPrimary} disabled={submitting}>
            {submitting ? 'Saving…' : 'Set password'}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
