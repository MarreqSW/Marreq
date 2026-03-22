/**
 * Status tag color picker: keeps a native color input and a hex/RGB text input in sync.
 * Used on requirement status and test status (new/edit) forms.
 * Containers must have class .status-tag-color-group with:
 *   - .status-tag-color-picker (input[type="color"])
 *   - .status-tag-color-hex (input[type="text"], name="tag_color")
 * Text input accepts: #RGB, #RRGGBB, or rgb(r,g,b).
 */

const HEX_REGEX = /^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$/;
const RGB_REGEX = /^rgb\s*\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*\)$/;
const THREE_NUMS_REGEX = /^\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*$/;

function parseHex(hex) {
  if (!hex || !HEX_REGEX.test(hex)) return null;
  const m = hex.match(/^#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6})$/);
  if (!m) return null;
  let s = m[1];
  if (s.length === 3) {
    s = s[0] + s[0] + s[1] + s[1] + s[2] + s[2];
  }
  return '#' + s;
}

function rgbToHex(r, g, b) {
  const clamp = (n) => Math.max(0, Math.min(255, parseInt(n, 10)));
  const rr = clamp(r).toString(16).padStart(2, '0');
  const gg = clamp(g).toString(16).padStart(2, '0');
  const bb = clamp(b).toString(16).padStart(2, '0');
  return '#' + rr + gg + bb;
}

function parseColorInput(value) {
  if (!value || typeof value !== 'string') return null;
  const trimmed = value.trim();
  if (HEX_REGEX.test(trimmed)) return parseHex(trimmed);
  const rgbMatch = trimmed.match(RGB_REGEX);
  if (rgbMatch) return rgbToHex(rgbMatch[1], rgbMatch[2], rgbMatch[3]);
  const threeMatch = trimmed.match(THREE_NUMS_REGEX);
  if (threeMatch) return rgbToHex(threeMatch[1], threeMatch[2], threeMatch[3]);
  return null;
}

export function initStatusColorPickers() {
  document.querySelectorAll('.status-tag-color-group').forEach((group) => {
    const picker = group.querySelector('.status-tag-color-picker');
    const hexInput = group.querySelector('.status-tag-color-hex');
    if (!picker || !hexInput) return;

    picker.addEventListener('input', () => {
      hexInput.value = picker.value;
      hexInput.dispatchEvent(new Event('input', { bubbles: true }));
    });

    hexInput.addEventListener('input', () => {
      const normalized = parseColorInput(hexInput.value);
      if (normalized) {
        picker.value = normalized;
      }
    });

    hexInput.addEventListener('change', () => {
      const trimmed = hexInput.value.trim();
      if (!trimmed) return;
      const normalized = parseColorInput(trimmed);
      if (normalized) {
        picker.value = normalized;
        hexInput.value = normalized;
      } else {
        const sixHex = /^([0-9A-Fa-f]{6})$/;
        if (sixHex.test(trimmed)) {
          const withHash = '#' + trimmed;
          hexInput.value = withHash;
          picker.value = withHash;
        }
      }
    });

    // Initial sync from hex to picker (edit page with existing color)
    const initial = hexInput.value.trim();
    if (initial) {
      const normalized = parseColorInput(initial);
      if (normalized) {
        picker.value = normalized;
        hexInput.value = normalized;
      }
    }
  });
}
