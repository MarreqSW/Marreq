import { useEffect, useState } from 'react';
import { createUser, updateUser } from '@/api/client';
import type { User } from '@/api/types';
import Dialog from '@/components/Dialog';
import { useFormSubmit } from '@/hooks/useFormSubmit';
import { btnPrimary, inp } from '@/pages/catalog/catalogUi';

type UserFormDialogProps = {
  open: boolean;
  /** User being edited; `null` creates a new account. */
  user: User | null;
  /** False when the deployment mode forbids changing the admin flag. */
  allowsAdminPromotion: boolean;
  /** True when editing the signed-in admin (cannot remove own admin rights). */
  isSelf: boolean;
  getCsrfToken: () => Promise<string>;
  onClose: () => void;
  onSaved: () => void;
};

const label = 'block text-xs font-semibold text-stitch-fg mb-1';

export default function UserFormDialog({
  open,
  user,
  allowsAdminPromotion,
  isSelf,
  getCsrfToken,
  onClose,
  onSaved,
}: UserFormDialogProps) {
  const creating = user === null;
  const [username, setUsername] = useState('');
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [isAdmin, setIsAdmin] = useState(false);
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  const { error, setError, submitting, onSubmit } = useFormSubmit(async () => {
    const csrf = await getCsrfToken();
    const profile = { username, name, email, is_admin: isAdmin };
    if (creating) {
      if (password !== confirmPassword) throw new Error('Passwords do not match');
      await createUser({ ...profile, password }, csrf);
    } else {
      await updateUser(user.id, profile, csrf);
    }
    onSaved();
  });

  useEffect(() => {
    if (!open) return;
    setUsername(user?.username ?? '');
    setName(user?.name ?? '');
    setEmail(user?.email ?? '');
    setIsAdmin(user?.is_admin ?? false);
    setPassword('');
    setConfirmPassword('');
    setError(null);
  }, [open, user, setError]);

  const adminLocked = !allowsAdminPromotion || (isSelf && isAdmin);

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={creating ? 'New user' : `Edit ${user.username}`}
      subtitle={creating ? 'Create an account that can sign in with a password.' : undefined}
    >
      <form onSubmit={onSubmit} className="space-y-4" data-testid="user-form">
        {error ? (
          <div
            role="alert"
            className="rounded-lg border border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100 text-sm px-4 py-2"
          >
            {error}
          </div>
        ) : null}
        <div>
          <label className={label} htmlFor="user-form-username">
            Username
          </label>
          <input
            id="user-form-username"
            className={inp}
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            autoFocus
            autoComplete="off"
          />
        </div>
        <div>
          <label className={label} htmlFor="user-form-name">
            Full name
          </label>
          <input
            id="user-form-name"
            className={inp}
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
        </div>
        <div>
          <label className={label} htmlFor="user-form-email">
            Email
          </label>
          <input
            id="user-form-email"
            type="email"
            className={inp}
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
        </div>
        {creating ? (
          <>
            <div>
              <label className={label} htmlFor="user-form-password">
                Password
              </label>
              <input
                id="user-form-password"
                type="password"
                className={inp}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                autoComplete="new-password"
              />
            </div>
            <div>
              <label className={label} htmlFor="user-form-confirm">
                Confirm password
              </label>
              <input
                id="user-form-confirm"
                type="password"
                className={inp}
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                required
                autoComplete="new-password"
              />
            </div>
          </>
        ) : null}
        <div>
          <label className="flex items-center gap-2 text-sm text-stitch-fg">
            <input
              type="checkbox"
              checked={isAdmin}
              onChange={(e) => setIsAdmin(e.target.checked)}
              disabled={adminLocked}
            />
            Site administrator
          </label>
          {!allowsAdminPromotion ? (
            <p className="mt-1 text-xs text-stitch-muted">
              Admin promotion is disabled in this deployment mode.
            </p>
          ) : isSelf && isAdmin ? (
            <p className="mt-1 text-xs text-stitch-muted">
              You cannot remove your own administrator rights.
            </p>
          ) : null}
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
            {submitting ? 'Saving…' : creating ? 'Create user' : 'Save'}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
