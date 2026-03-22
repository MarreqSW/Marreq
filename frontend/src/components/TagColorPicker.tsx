/**
 * Native color input + hex text field for status `tag_color` (#RRGGBB).
 */
export default function TagColorPicker({
  value,
  onChange,
  disabled,
  className = '',
  inputClassName,
}: {
  value: string | null | undefined;
  onChange: (next: string | null) => void;
  disabled?: boolean;
  className?: string;
  /** Tailwind classes for the text field (defaults to catalog input style). */
  inputClassName?: string;
}) {
  const raw = (value ?? '').trim();
  const validHex = /^#[0-9A-Fa-f]{6}$/.test(raw);
  /** `type="color"` requires a full hex; use neutral when unset or invalid. */
  const swatchValue = validHex ? raw : '#64748b';

  const defaultInp =
    'w-full min-w-0 text-sm bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-white focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none disabled:opacity-45';

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <input
        type="color"
        aria-label="Color"
        title="Pick color"
        className="h-9 w-11 shrink-0 cursor-pointer rounded-md border border-stitch-border bg-stitch-elevated p-0 disabled:cursor-not-allowed disabled:opacity-45 [color-scheme:dark]"
        value={swatchValue}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
      />
      <input
        type="text"
        className={inputClassName ?? defaultInp}
        placeholder="#RRGGBB"
        value={raw}
        disabled={disabled}
        spellCheck={false}
        onChange={(e) => {
          const v = e.target.value.trim();
          onChange(v === '' ? null : v);
        }}
      />
    </div>
  );
}
