import type { DraftSaveStatus } from '@/hooks/useDraftAutosave';
import { formatDraftTime } from '@/utils/requirementDraft';

type Props = {
  status: DraftSaveStatus;
  savedAt: Date | null;
  dirty: boolean;
  /** The requirement itself is being saved to the server. */
  saving?: boolean;
};

/**
 * Footer status of a requirement editor (issue #255). The local draft is
 * named as such, so it is not mistaken for a saved version.
 */
export default function DraftStatus({ status, savedAt, dirty, saving = false }: Props) {
  let text: string | null = null;
  let tone = 'text-stitch-muted';
  let icon = '';
  if (saving) {
    text = 'Saving…';
    icon = 'progress_activity';
  } else if (status === 'error' && dirty) {
    text = "Couldn't keep a draft on this device";
    tone = 'text-amber-800 dark:text-amber-200';
    icon = 'warning';
  } else if (status === 'saved' && dirty && savedAt) {
    text = `Draft kept on this device · ${formatDraftTime(savedAt)}`;
    icon = 'cloud_off';
  } else if (dirty) {
    text = 'Unsaved changes';
    icon = 'edit_note';
  }

  return (
    <p
      aria-live="polite"
      data-testid="draft-status"
      className={`flex items-center gap-1.5 text-[11px] font-medium ${tone}`}
      title={
        status === 'saved' && dirty
          ? 'Your changes are kept in this browser until you save or discard them. Only Save creates a new version.'
          : undefined
      }
    >
      {text ? (
        <>
          <span aria-hidden className="material-symbols-outlined text-base leading-none">
            {icon}
          </span>
          {text}
        </>
      ) : null}
    </p>
  );
}
