import { type DragEvent, useCallback, useEffect, useRef, useState } from 'react';
import {
  ApiError,
  deleteAttachment,
  downloadAttachment,
  getProjectStorage,
  listAttachments,
  uploadAttachment,
} from '@/api/client';
import type { Attachment, AttachmentEntityType, ProjectStorage } from '@/api/types';
import { formatBytes } from '@/utils/formatBytes';

type Props = {
  projectId: number;
  entityType: AttachmentEntityType;
  entityId: number;
  /** `edit_requirements` (and not a historical view). */
  canEdit: boolean;
  csrfToken: string;
  className?: string;
};

function iconFor(contentType: string): string {
  if (contentType === 'application/pdf') return 'picture_as_pdf';
  if (contentType.startsWith('image/')) return 'image';
  if (contentType.includes('spreadsheet') || contentType === 'text/csv') return 'table_chart';
  if (contentType.includes('presentation')) return 'slideshow';
  if (contentType === 'application/zip') return 'folder_zip';
  return 'description';
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(undefined, { dateStyle: 'medium' });
}

function extensionOf(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : '';
}

/** Why the browser can already tell this file will be refused, if it can. */
export function precheckFile(file: File, storage: ProjectStorage | null): string | null {
  if (!storage) return null;
  if (file.size === 0) return `${file.name} is empty.`;
  if (file.size > storage.max_file_bytes) {
    return `${file.name} is ${formatBytes(file.size)}; files can be at most ${formatBytes(storage.max_file_bytes)}.`;
  }
  const ext = extensionOf(file.name);
  if (!storage.allowed_extensions.includes(ext)) {
    return `${file.name}: ${ext ? `.${ext} files are` : 'files without an extension are'} not allowed.`;
  }
  return null;
}

function errorMessage(err: unknown, file: File): string {
  if (err instanceof ApiError) return `${file.name}: ${err.message}`;
  return `${file.name}: ${err instanceof Error ? err.message : 'upload failed'}`;
}

/**
 * Files attached to a requirement or verification: list, download, upload
 * (button or drop), delete, plus the project's storage use. Buttons use
 * `type="button"` because the panel sits inside edit forms.
 */
export default function AttachmentsPanel({
  projectId,
  entityType,
  entityId,
  canEdit,
  csrfToken,
  className = '',
}: Props) {
  const [items, setItems] = useState<Attachment[] | null>(null);
  const [storage, setStorage] = useState<ProjectStorage | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [errors, setErrors] = useState<string[]>([]);
  const [uploading, setUploading] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const refreshStorage = useCallback(async () => {
    setStorage((await Promise.resolve(getProjectStorage(projectId)).catch(() => null)) ?? null);
  }, [projectId]);

  useEffect(() => {
    let cancelled = false;
    setItems(null);
    setLoadError(null);
    Promise.all([
      listAttachments(projectId, entityType, entityId),
      Promise.resolve(getProjectStorage(projectId)).catch(() => null),
    ])
      .then(([list, st]) => {
        if (cancelled) return;
        setItems(list ?? []);
        setStorage(st ?? null);
      })
      .catch((err: unknown) => {
        if (!cancelled) setLoadError(err instanceof Error ? err.message : 'Could not load attachments');
      });
    return () => {
      cancelled = true;
    };
  }, [projectId, entityType, entityId]);

  async function uploadFiles(files: File[]) {
    if (!canEdit || files.length === 0) return;
    const problems: string[] = [];
    for (const file of files) {
      const precheck = precheckFile(file, storage);
      if (precheck) {
        problems.push(precheck);
        continue;
      }
      setUploading(file.name);
      try {
        const created = await uploadAttachment(projectId, entityType, entityId, file, csrfToken);
        setItems((prev) => [...(prev ?? []), created]);
      } catch (err) {
        problems.push(errorMessage(err, file));
      }
    }
    setUploading(null);
    setErrors(problems);
    await refreshStorage();
  }

  async function remove(attachment: Attachment) {
    if (!window.confirm(`Delete ${attachment.filename}?`)) return;
    try {
      await deleteAttachment(projectId, attachment.id, csrfToken);
      setItems((prev) => (prev ?? []).filter((a) => a.id !== attachment.id));
      setErrors([]);
      await refreshStorage();
    } catch (err) {
      setErrors([err instanceof Error ? err.message : 'Could not delete the file']);
    }
  }

  async function download(attachment: Attachment) {
    try {
      await downloadAttachment(projectId, attachment);
    } catch (err) {
      setErrors([err instanceof Error ? err.message : 'Could not download the file']);
    }
  }

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragOver(false);
    void uploadFiles(Array.from(e.dataTransfer.files));
  }

  const usedPct = storage
    ? Math.min(100, Math.round((storage.used_bytes / Math.max(1, storage.quota_bytes)) * 100))
    : 0;
  const accept = storage?.allowed_extensions.map((e) => `.${e}`).join(',');

  return (
    <section
      aria-label="Attachments"
      className={`bg-stitch-surface rounded-xl border border-stitch-border p-6 shadow-stitch ${className}`}
    >
      <div className="flex items-center justify-between gap-2 mb-4">
        <div className="flex items-center gap-2">
          <span className="material-symbols-outlined text-stitch-accent text-xl">attach_file</span>
          <h2 className="text-sm font-bold font-headline text-stitch-accent">Attachments</h2>
          {items && items.length > 0 ? (
            <span className="rounded-full bg-stitch-elevated px-2 text-[10px] font-bold text-stitch-muted">
              {items.length}
            </span>
          ) : null}
        </div>
        {canEdit ? (
          <button
            type="button"
            onClick={() => inputRef.current?.click()}
            disabled={uploading !== null}
            className="inline-flex items-center gap-1 text-xs font-bold text-stitch-accent hover:underline disabled:opacity-50"
          >
            <span className="material-symbols-outlined text-base" aria-hidden>
              upload_file
            </span>
            Add files
          </button>
        ) : null}
      </div>

      {loadError ? <p className="text-xs text-red-400">{loadError}</p> : null}
      {!items && !loadError ? <p className="text-xs text-stitch-muted">Loading…</p> : null}

      {items ? (
        items.length === 0 ? (
          <p className="text-xs text-stitch-muted">No files attached.</p>
        ) : (
          <ul className="space-y-2">
            {items.map((a) => (
              <li key={a.id} className="flex items-start gap-2 group">
                <span className="material-symbols-outlined text-stitch-muted text-lg mt-0.5" aria-hidden>
                  {iconFor(a.content_type)}
                </span>
                <div className="min-w-0 flex-1">
                  <button
                    type="button"
                    onClick={() => void download(a)}
                    className="block max-w-full truncate text-left text-xs font-semibold text-stitch-fg hover:text-stitch-accent hover:underline"
                    title={`Download ${a.filename}`}
                  >
                    {a.filename}
                  </button>
                  <p className="text-[10px] text-stitch-muted">
                    {formatBytes(a.size_bytes)}
                    {a.uploaded_by_name ? ` · ${a.uploaded_by_name}` : ''} · {formatDate(a.created_at)}
                  </p>
                </div>
                {canEdit ? (
                  <button
                    type="button"
                    onClick={() => void remove(a)}
                    aria-label={`Delete ${a.filename}`}
                    className="material-symbols-outlined text-base text-stitch-muted hover:text-red-400 opacity-60 group-hover:opacity-100"
                  >
                    delete
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        )
      ) : null}

      {canEdit && items ? (
        <div
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragLeave={() => setDragOver(false)}
          onDrop={onDrop}
          className={`mt-4 rounded-lg border border-dashed px-3 py-3 text-center text-[11px] transition-colors ${
            dragOver ? 'border-stitch-accent bg-stitch-accent/10 text-stitch-fg' : 'border-stitch-border text-stitch-muted'
          }`}
        >
          {uploading ? (
            <span className="inline-flex items-center gap-1" role="status">
              <span className="material-symbols-outlined text-sm animate-spin" aria-hidden>
                progress_activity
              </span>
              Uploading {uploading}…
            </span>
          ) : (
            <>
              Drop files here
              {storage ? ` · up to ${formatBytes(storage.max_file_bytes)} each` : ''}
            </>
          )}
          <input
            ref={inputRef}
            type="file"
            multiple
            accept={accept}
            aria-label="Upload attachments"
            className="hidden"
            onChange={(e) => {
              const files = Array.from(e.target.files ?? []);
              e.target.value = '';
              void uploadFiles(files);
            }}
          />
        </div>
      ) : null}

      {errors.length > 0 ? (
        <ul role="alert" className="mt-3 space-y-1 text-[11px] text-red-400">
          {errors.map((msg) => (
            <li key={msg}>{msg}</li>
          ))}
        </ul>
      ) : null}

      {storage ? (
        <div className="mt-4">
          <div className="h-1.5 rounded-full bg-stitch-elevated overflow-hidden" aria-hidden>
            <div
              className={`h-full ${usedPct >= 90 ? 'bg-red-400' : 'bg-stitch-accent'}`}
              style={{ width: `${usedPct}%` }}
            />
          </div>
          <p className="mt-1 text-[10px] text-stitch-muted">
            Project storage: {formatBytes(storage.used_bytes)} of {formatBytes(storage.quota_bytes)} used
          </p>
        </div>
      ) : null}
    </section>
  );
}
