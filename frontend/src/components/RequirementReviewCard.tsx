import { Link } from 'react-router-dom';
import type { Requirement, RequirementStatus, VerificationMethod } from '@/api/types';
import { preventEditNavigationIfUnconfirmed } from '@/utils/confirmEditApprovedRequirement';
import { StatusBadge } from './StatusBadge';

function approvalLabel(state: string): string {
  return state.replace(/_/g, ' ').toUpperCase();
}

function verificationMethodsText(ids: number[], methods: VerificationMethod[]): string {
  if (!ids.length) return 'Not specified';
  return ids
    .map((id) => methods.find((method) => method.id === id)?.title?.trim() || `#${id}`)
    .join(', ');
}

function parentRequirementIds(req: Requirement): number[] {
  if (req.parent_requirement_ids?.length) return req.parent_requirement_ids;
  return req.parent_id == null ? [] : [req.parent_id];
}

function formatModified(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  }).format(date);
}

export default function RequirementReviewCard({
  requirement,
  basePath,
  status,
  category,
  author,
  verificationMethods,
  requirementKeyById,
  requirementTitleById,
  canEdit,
}: {
  requirement: Requirement;
  basePath: string;
  status?: RequirementStatus;
  category: string;
  author: string;
  verificationMethods: VerificationMethod[];
  requirementKeyById: ReadonlyMap<number, string>;
  requirementTitleById: ReadonlyMap<number, string>;
  canEdit: boolean;
}) {
  const parentIds = parentRequirementIds(requirement);
  const statusTitle = status?.title ?? `Status #${requirement.status_id}`;
  const statement = requirement.description.trim();

  return (
    <article
      aria-labelledby={`requirement-${requirement.id}-title`}
      className="group rounded-xl border border-stitch-border bg-stitch-surface shadow-stitch transition-colors hover:border-stitch-accent/30 hover:bg-white/[0.025]"
    >
      <div className="p-4 sm:p-5">
        <header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <div className="mb-1.5 flex flex-wrap items-center gap-2">
              <Link
                to={`${basePath}/requirements/${requirement.id}`}
                className="font-mono text-xs font-bold tracking-wide text-stitch-accent hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-stitch-accent"
              >
                {requirement.reference_code || `REQ-${requirement.id}`}
              </Link>
              <StatusBadge title={statusTitle} tagColor={status?.tag_color} />
              <span className="rounded border border-stitch-border px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide text-stitch-muted">
                {approvalLabel(requirement.approval_state)}
              </span>
              <span className="rounded bg-stitch-elevated px-2 py-0.5 text-[11px] font-medium text-stitch-fg/80">
                {category}
              </span>
            </div>
            <h3
              id={`requirement-${requirement.id}-title`}
              className="text-base font-bold leading-snug text-stitch-fg sm:text-lg"
            >
              {requirement.title.trim() || 'Untitled requirement'}
            </h3>
          </div>

          <nav
            aria-label={`Actions for ${requirement.reference_code || requirement.title}`}
            className="flex shrink-0 items-center gap-1"
          >
            <Link
              to={`${basePath}/requirements/${requirement.id}`}
              className="inline-flex items-center gap-1 rounded-md px-2 py-1.5 text-xs font-semibold text-stitch-muted hover:bg-stitch-higher hover:text-stitch-accent focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-stitch-accent"
            >
              <span aria-hidden="true" className="material-symbols-outlined text-base">
                visibility
              </span>
              View
            </Link>
            <Link
              to={`${basePath}/requirements/${requirement.id}/edit`}
              onClick={(event) =>
                preventEditNavigationIfUnconfirmed(
                  event,
                  requirement.approval_state,
                  requirement.id,
                )
              }
              className="inline-flex items-center gap-1 rounded-md px-2 py-1.5 text-xs font-semibold text-stitch-muted hover:bg-stitch-higher hover:text-stitch-accent focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-stitch-accent"
            >
              <span aria-hidden="true" className="material-symbols-outlined text-base">
                edit
              </span>
              Edit
            </Link>
            {canEdit ? (
              <Link
                to={`${basePath}/requirements/new?from=${requirement.id}`}
                aria-label={`Duplicate ${requirement.reference_code || requirement.title}`}
                className="inline-flex rounded-md p-1.5 text-stitch-muted hover:bg-stitch-higher hover:text-stitch-accent focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-stitch-accent"
                title="Duplicate"
              >
                <span aria-hidden="true" className="material-symbols-outlined text-base">
                  content_copy
                </span>
              </Link>
            ) : null}
          </nav>
        </header>

        <p
          className={`mt-3 max-w-5xl text-sm leading-6 ${statement ? 'line-clamp-3 text-stitch-fg/85' : 'italic text-stitch-muted'}`}
          title={statement || undefined}
        >
          {statement || 'No requirement statement provided.'}
        </p>

        <dl className="mt-4 grid grid-cols-1 gap-x-6 gap-y-2 border-t border-stitch-border/60 pt-3 text-xs sm:grid-cols-2 lg:grid-cols-4">
          <div className="flex min-w-0 items-baseline gap-2">
            <dt className="shrink-0 text-stitch-muted">Parent</dt>
            <dd className="min-w-0 text-stitch-fg/85">
              {parentIds.length ? (
                <span className="flex flex-wrap gap-x-2 gap-y-1">
                  {parentIds.map((parentId) => (
                    <Link
                      key={parentId}
                      to={`${basePath}/requirements/${parentId}`}
                      className="truncate font-mono text-stitch-accent hover:underline"
                      title={requirementTitleById.get(parentId)}
                    >
                      {requirementKeyById.get(parentId) ?? `REQ-${parentId}`}
                    </Link>
                  ))}
                </span>
              ) : (
                <span className="text-stitch-muted">None</span>
              )}
            </dd>
          </div>
          <div className="flex min-w-0 items-baseline gap-2">
            <dt className="shrink-0 text-stitch-muted">Verification</dt>
            <dd
              className="truncate text-stitch-fg/85"
              title={verificationMethodsText(
                requirement.verification_method_ids ?? [],
                verificationMethods,
              )}
            >
              {verificationMethodsText(
                requirement.verification_method_ids ?? [],
                verificationMethods,
              )}
            </dd>
          </div>
          <div className="flex min-w-0 items-baseline gap-2">
            <dt className="shrink-0 text-stitch-muted">Author</dt>
            <dd className="truncate text-stitch-fg/85" title={author}>
              {author}
            </dd>
          </div>
          <div className="flex min-w-0 items-baseline gap-2 sm:justify-start lg:justify-end">
            <dt className="shrink-0 text-stitch-muted">Modified</dt>
            <dd
              className="whitespace-nowrap font-mono text-stitch-fg/85"
              title={requirement.update_date}
            >
              {formatModified(requirement.update_date)}
            </dd>
          </div>
        </dl>
      </div>
    </article>
  );
}
