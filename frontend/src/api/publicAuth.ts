import type { ForgotPasswordBody, RegistrationBody, ResetPasswordBody } from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function registerAccount(body: RegistrationBody): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/register', {
    method: 'POST',
    headers: JSON_HEADERS,
    body: JSON.stringify(body),
  });
}

export async function verifyEmail(token: string): Promise<void> {
  await fetchJson<{ status: string }>(
    `/api/auth/verify-email?token=${encodeURIComponent(token)}`,
  );
}

export async function requestPasswordReset(body: ForgotPasswordBody): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/forgot-password', {
    method: 'POST',
    headers: JSON_HEADERS,
    body: JSON.stringify(body),
  });
}

export async function resetPassword(body: ResetPasswordBody): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/reset-password', {
    method: 'POST',
    headers: JSON_HEADERS,
    body: JSON.stringify(body),
  });
}
