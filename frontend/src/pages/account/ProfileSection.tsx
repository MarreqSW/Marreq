import { useEffect, useRef, useState } from 'react';
import { updateMyProfile } from '@/api/client';
import type { User } from '@/api/types';
import { useFormSubmit } from '@/hooks/useFormSubmit';

type ProfileSectionProps = {
  user: User;
  /** False where emails must be verified (hosted cloud): the email is read-only. */
  emailEditable: boolean;
  /** The account has a password, so an email change needs it. */
  passwordConfigured: boolean;
  getCsrfToken: () => Promise<string>;
  onSaved: (user: User) => void;
};

const label = 'block text-xs font-semibold text-stitch-muted uppercase mb-1';
const input =
  'w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-hidden focus:ring-2 focus:ring-stitch-accent/50 read-only:opacity-70';

/** Display name and email editor for the signed-in user (username stays fixed). */
export default function ProfileSection({
  user,
  emailEditable,
  passwordConfigured,
  getCsrfToken,
  onSaved,
}: ProfileSectionProps) {
  const [name, setName] = useState(user.name);
  const [email, setEmail] = useState(user.email);
  const [currentPassword, setCurrentPassword] = useState('');
  const [saved, setSaved] = useState(false);

  // Resync the fields when a different user object arrives (e.g. after saving),
  // but not on mount: the initial state already comes from `user`, and a mount-time
  // reset would overwrite anything typed before the effect runs.
  const syncedUser = useRef(user);
  useEffect(() => {
    if (syncedUser.current === user) return;
    syncedUser.current = user;
    setName(user.name);
    setEmail(user.email);
  }, [user]);

  const emailChanged = email.trim().toLowerCase() !== user.email.trim().toLowerCase();

  const { error, submitting, onSubmit } = useFormSubmit(async () => {
    setSaved(false);
    const updated = await updateMyProfile(
      {
        name,
        email,
        ...(emailChanged && currentPassword ? { current_password: currentPassword } : {}),
      },
      await getCsrfToken(),
    );
    setCurrentPassword('');
    setSaved(true);
    onSaved(updated);
  });

  return (
    <form onSubmit={onSubmit} className="space-y-3" aria-labelledby="profile-heading">
      <h2 id="profile-heading" className="text-sm font-semibold text-stitch-fg">
        Profile
      </h2>
      {error ? (
        <div
          role="alert"
          className="rounded-lg bg-red-500/10 border border-red-500/25 px-3 py-2 text-sm text-red-800 dark:text-red-200"
        >
          {error}
        </div>
      ) : null}
      {saved ? (
        <div
          role="status"
          className="rounded-lg bg-emerald-500/10 border border-emerald-500/25 px-3 py-2 text-sm text-emerald-900 dark:text-emerald-100"
        >
          Profile updated.
        </div>
      ) : null}
      <div>
        <label htmlFor="profile-username" className={label}>
          Username
        </label>
        <input id="profile-username" className={input} value={user.username} readOnly />
        <p className="mt-1 text-xs text-stitch-muted">
          Your username is also your workspace path and can&apos;t be changed here.
        </p>
      </div>
      <div>
        <label htmlFor="profile-name" className={label}>
          Full name
        </label>
        <input
          id="profile-name"
          className={input}
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            setSaved(false);
          }}
          autoComplete="name"
          required
        />
      </div>
      <div>
        <label htmlFor="profile-email" className={label}>
          Email
        </label>
        <input
          id="profile-email"
          type="email"
          className={input}
          value={email}
          onChange={(e) => {
            setEmail(e.target.value);
            setSaved(false);
          }}
          readOnly={!emailEditable}
          autoComplete="email"
          required
        />
        {!emailEditable ? (
          <p className="mt-1 text-xs text-stitch-muted">
            Email changes are not available on this service; contact your administrator.
          </p>
        ) : null}
      </div>
      {emailEditable && emailChanged ? (
        <div>
          <label htmlFor="profile-current-password" className={label}>
            Current password
          </label>
          <input
            id="profile-current-password"
            type="password"
            className={input}
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            autoComplete="current-password"
            required={passwordConfigured}
          />
          <p className="mt-1 text-xs text-stitch-muted">
            {passwordConfigured
              ? 'Confirm your password to change your email.'
              : 'Leave empty if you only sign in with an external account.'}
          </p>
        </div>
      ) : null}
      <button
        type="submit"
        disabled={submitting}
        className="w-full rounded-lg bg-linear-to-br from-primary to-primary-container text-white font-semibold py-2.5 text-sm hover:opacity-95 disabled:opacity-60"
      >
        {submitting ? 'Saving…' : 'Save profile'}
      </button>
    </form>
  );
}
