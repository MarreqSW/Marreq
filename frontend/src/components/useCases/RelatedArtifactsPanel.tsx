import type { UseCaseLink, UseCaseLabels } from '@/api/types';

type Props = {
  links: UseCaseLink[];
  labels: UseCaseLabels;
  canEdit: boolean;
  onAdd: () => void;
  onOpen: (link: UseCaseLink) => void;
  onRemove: (link: UseCaseLink) => void;
  onClear: (link: UseCaseLink) => void;
};

function Row({ link, labels, canEdit, onOpen, onRemove, onClear }: Omit<Props, 'links' | 'onAdd'> & { link: UseCaseLink }) {
  const where = link.element_id
    ? labels[link.element_id] ?? link.element_label ?? link.element_id
    : null;
  return (
    <li className="border-t border-stitch-border py-2 first:border-t-0">
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => onOpen(link)}
          className="font-mono text-[11px] font-semibold text-stitch-accent hover:underline"
        >
          {link.target_reference_code}
        </button>
        <span className="ml-auto text-[10px] text-stitch-muted">
          {where ? (/^\d/.test(where) ? `step ${where}` : where) : 'whole use case'}
        </span>
        {canEdit ? (
          <button
            type="button"
            title="Remove link"
            aria-label={`Remove link to ${link.target_reference_code}`}
            onClick={() => onRemove(link)}
            className="rounded-sm text-stitch-muted hover:text-stitch-danger"
          >
            <span className="material-symbols-outlined text-sm">close</span>
          </button>
        ) : null}
      </div>
      <p className="text-xs text-stitch-fg">{link.target_title}</p>
      {link.rationale ? <p className="text-[11px] italic text-stitch-muted">{link.rationale}</p> : null}
      {link.suspect ? (
        <div className="mt-1 flex items-center gap-1.5 rounded-md bg-amber-500/10 px-2 py-1 text-[11px] text-amber-800 dark:text-amber-300">
          <span className="material-symbols-outlined text-sm">link_off</span>
          <span>Changed since linked{link.suspect_reason ? ` · ${link.suspect_reason}` : ''}</span>
          {canEdit ? (
            <button type="button" onClick={() => onClear(link)} className="ml-auto font-semibold hover:underline">
              Clear
            </button>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

/** Requirements and verifications linked to the use case (screens 2 and 7 of #388). */
export default function RelatedArtifactsPanel(props: Props) {
  const { links, canEdit, onAdd } = props;
  const requirements = links.filter((l) => l.target_type === 'requirement');
  const verifications = links.filter((l) => l.target_type === 'verification');
  return (
    <div className="rounded-lg border border-stitch-border bg-stitch-surface shadow-sm">
      <div className="flex items-center gap-2 border-b border-stitch-border px-4 py-2.5">
        <span className="material-symbols-outlined text-lg text-stitch-accent">hub</span>
        <h3 className="font-headline text-sm font-bold text-stitch-accent">Related artifacts</h3>
        {canEdit ? (
          <button
            type="button"
            onClick={onAdd}
            title="Link a requirement or verification"
            aria-label="Link a requirement or verification"
            className="ml-auto rounded-sm p-0.5 text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-accent"
          >
            <span className="material-symbols-outlined text-base">add_link</span>
          </button>
        ) : null}
      </div>
      <div className="px-4 py-2 text-xs">
        <p className="mt-1 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">
          Requirements ({requirements.length})
        </p>
        {requirements.length ? (
          <ul>{requirements.map((l) => <Row key={l.id} {...props} link={l} />)}</ul>
        ) : (
          <p className="py-1.5 text-stitch-muted">None linked.</p>
        )}
        <p className="mt-3 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">
          Verifications ({verifications.length})
        </p>
        {verifications.length ? (
          <ul>{verifications.map((l) => <Row key={l.id} {...props} link={l} />)}</ul>
        ) : (
          <p className="py-1.5 text-stitch-muted">None linked.</p>
        )}
        <p className="mt-2 border-t border-stitch-border pt-2 text-[10px] text-stitch-muted">
          Linking a verification here does not mark the requirements as verified.
        </p>
      </div>
    </div>
  );
}
