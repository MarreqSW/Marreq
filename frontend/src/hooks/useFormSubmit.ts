import { FormEvent, useState } from 'react';

/**
 * Encapsulates the shared submit/error/submitting state pattern used across auth forms.
 * `action` should throw an Error on failure; the message is captured into `error`.
 */
export function useFormSubmit(action: (e: FormEvent) => Promise<void>): {
  error: string | null;
  setError: (msg: string | null) => void;
  submitting: boolean;
  onSubmit: (e: FormEvent) => Promise<void>;
} {
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await action(e);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An unexpected error occurred');
    } finally {
      setSubmitting(false);
    }
  }

  return { error, setError, submitting, onSubmit };
}
