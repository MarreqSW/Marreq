const COLOURS = [
  'bg-indigo-700',
  'bg-teal-700',
  'bg-amber-700',
  'bg-sky-700',
  'bg-rose-700',
  'bg-emerald-700',
  'bg-violet-700',
  'bg-slate-600',
];

const SIZES = {
  xs: 'w-4 h-4 text-[9px] rounded-sm',
  sm: 'w-6 h-6 text-[11px] rounded-md',
  md: 'w-8 h-8 text-xs rounded-lg',
} as const;

/** A project's first letter on a colour derived from its id, so it stays the same everywhere. */
export default function ProjectInitial({
  id,
  name,
  size = 'sm',
}: {
  id: number;
  name: string;
  size?: keyof typeof SIZES;
}) {
  const colour = COLOURS[Math.abs(id) % COLOURS.length];
  return (
    <span
      aria-hidden
      className={`${SIZES[size]} ${colour} text-white font-bold flex items-center justify-center shrink-0`}
    >
      {(name.trim()[0] ?? '?').toUpperCase()}
    </span>
  );
}
