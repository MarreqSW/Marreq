import type { ReactNode } from 'react';
import { useResizableGraphHeight } from '@/hooks/useResizableGraphHeight';

type ResizableGraphShellProps = {
  projectId: number;
  viewKey: string;
  children: ReactNode;
  /** When true, shell uses flex centering for loading/empty placeholders. */
  placeholder?: boolean;
};

export default function ResizableGraphShell({
  projectId,
  viewKey,
  children,
  placeholder = false,
}: ResizableGraphShellProps) {
  const { heightPx, containerStyle, onResizeStart } = useResizableGraphHeight(
    projectId,
    viewKey,
  );

  return (
    <div className="relative w-full" style={containerStyle}>
      <div
        className={`stitch-flow w-full border border-stitch-border rounded-xl bg-stitch-surface overflow-hidden shadow-stitch ${
          placeholder ? 'flex items-center justify-center' : 'h-full'
        }`}
        style={placeholder ? containerStyle : { height: '100%', minHeight: 0 }}
      >
        {children}
      </div>
      <button
        type="button"
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize graph height"
        aria-valuenow={heightPx}
        aria-valuemin={320}
        aria-valuemax={1200}
        title="Drag to resize graph height"
        onMouseDown={onResizeStart}
        className="absolute left-0 right-0 bottom-0 h-2 cursor-ns-resize z-10 border-0 bg-transparent p-0 hover:bg-stitch-accent/25 active:bg-stitch-accent/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-stitch-accent/50 rounded-b-xl"
      />
    </div>
  );
}
