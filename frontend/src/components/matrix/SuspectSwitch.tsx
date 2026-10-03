type Props = {
  checked: boolean;
  onChange: (checked: boolean) => void;
};

/** Matrix toolbar: the yes/no "Suspect only" filter, as a switch (issue #361). */
export default function SuspectSwitch({ checked, onChange }: Props) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      title="Show only suspect links (rows and columns filtered)"
      onClick={() => onChange(!checked)}
      className="group inline-flex items-center gap-2 rounded-md px-1.5 py-1 text-[11px] font-medium text-stitch-fg hover:bg-stitch-higher focus:outline-hidden focus-visible:ring-2 focus-visible:ring-stitch-accent"
    >
      <span
        aria-hidden
        className={`relative inline-flex h-4 w-7 shrink-0 items-center rounded-full transition-colors ${
          checked ? 'bg-stitch-accent' : 'bg-stitch-border'
        }`}
      >
        <span
          className={`absolute h-3 w-3 rounded-full bg-white shadow-sm transition-transform ${
            checked ? 'translate-x-3.5' : 'translate-x-0.5'
          }`}
        />
      </span>
      <span className="h-3 w-3 rounded-sm shadow-[inset_0_0_0_2px_var(--color-stitch-danger)]" aria-hidden />
      Suspect only
    </button>
  );
}
