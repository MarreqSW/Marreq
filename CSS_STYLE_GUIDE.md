# 🧹 CSS Style Guide — Marreq Stylelint Rules

This repository uses **Stylelint** to enforce consistent, maintainable, and modern CSS conventions.
The configuration lives in `.stylelintrc` and extends the official [`stylelint-config-standard`](https://github.com/stylelint/stylelint-config-standard).

---

## Overview

Our Stylelint rules enforce:

* Predictable class naming (BEM + prefixes)
* Low selector specificity for easy overrides
* Shallow nesting for readability
* Alphabetical property order
* Strict avoidance of deprecated or redundant CSS

The goal is clean, scalable, and conflict-free styles across all components.

---

## 🧩 Class Naming Convention

### Rule: `selector-class-pattern`

```regex
^(?:marreq|c|o|u|is|has|status|js)-[a-z0-9]+(?:-[a-z0-9]+)*(?:__(?:[a-z0-9]+(?:-[a-z0-9]+)*))?(?:--[a-z0-9]+(?:-[a-z0-9]+)*)?$
```

Classes must:

* Use **kebab-case** (lowercase with hyphens)
* Optionally follow **BEM** (`block__element--modifier`)
* Begin with one of these **prefixes**:

| Prefix    | Meaning                               | Example                     |
| --------- | ------------------------------------- | --------------------------- |
| `marreq-` | Project-wide namespace                | `marreq-header`             |
| `c-`      | Component                             | `c-button`, `c-card__title` |
| `o-`      | Object (layout or structural utility) | `o-grid`                    |
| `u-`      | Utility class                         | `u-hidden`                  |
| `is-`     | State (boolean modifier)              | `is-active`                 |
| `has-`    | Context modifier                      | `has-error`                 |
| `status-` | Status-related block                  | `status-loading`            |
| `js-`     | JavaScript hook (non-styling)         | `js-toggle`                 |

**Valid examples:**

```css
.c-button { ... }
.c-button__icon { ... }
.c-button--disabled { ... }
.is-active { ... }
```

**Invalid examples:**

```css
.button { ... }          /* missing prefix */
.cButton { ... }         /* camelCase not allowed */
.c-button__Icon { ... }  /* uppercase not allowed */
```

---

## ⚖️ Selector Rules

| Rule                                 | Description                                             |
| ------------------------------------ | ------------------------------------------------------- |
| `selector-max-specificity: "0,3,0"`  | Keeps selectors lightweight; prevents specificity wars. |
| `selector-max-id: 0`                 | IDs cannot be used in selectors.                        |
| `selector-no-qualifying-type: true`  | Avoid `button.btn`; use only class selectors.           |
| `selector-max-compound-selectors: 4` | Limits complexity of chained selectors.                 |
| `selector-max-universal: 0`          | Disallows the universal `*` selector.                   |

---

## 💅 Declaration Rules

| Rule                                                       | Description                                                                |
| ---------------------------------------------------------- | -------------------------------------------------------------------------- |
| `declaration-no-important: true`                           | Bans `!important`; use proper specificity.                                 |
| `order/properties-alphabetical-order: true`                | Enforces alphabetical order for properties.                                |
| `max-nesting-depth: 3`                                     | Keeps nested selectors shallow (especially in SCSS).                       |
| `no-duplicate-selectors: true`                             | Avoids duplicate selector definitions.                                     |
| `color-hex-length: "short"`                                | Uses short hex (`#fff` instead of `#ffffff`).                              |
| `alpha-value-notation: "number"`                           | Uses numeric alpha values (`0.5` not `50%`).                               |
| `value-keyword-case: "lower"`                              | Enforces lowercase keywords (`block`, `none`).                             |
| `length-zero-no-unit: true`                                | Forbids units on zero values (`0px → 0`).                                  |
| `font-weight-notation: "numeric"`                          | Uses numeric font weights (`400`, `700`).                                  |
| `shorthand-property-no-redundant-values: true`             | Removes duplicate shorthand values (`margin: 10px 10px` → `margin: 10px`). |
| `declaration-block-no-redundant-longhand-properties: true` | Prefer shorthands where safe.                                              |
| `color-named: "never"`                                     | Disallows color names like `white` or `red`. Use tokens or hex instead.    |

---

## 🧠 Disabled / Relaxed Rules

| Rule                                                                            | Reason                                                   |
| ------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `no-descending-specificity: null`                                               | Disabled to avoid false positives with BEM.              |
| `property-no-vendor-prefix: null`                                               | We rely on Autoprefixer, not Stylelint, for prefixing.   |
| `media-feature-range-notation: null`                                            | Accepts both old (`min-width`) and new range notation.   |
| Formatting rules (`comment-empty-line-before`, `declaration-empty-line-before`) | Delegated to Prettier.                                   |
| Deprecated-property checks (`property-no-deprecated`, etc.)                     | Disabled until full browser support matrix is finalized. |

---

## 🚫 Ignored Files

```json
"ignoreFiles": ["**/_graveyard.css"]
```

Any CSS file placed in a `_graveyard.css` file is excluded from linting — useful for deprecated or experimental code.

---

## 💡 Rationale

This configuration promotes **clarity, consistency, and maintainability**:

* Predictable class names allow global search and refactoring.
* Strict specificity limits ensure easy overrides.
* Alphabetical ordering keeps diffs clean and removes “style arguments.”
* BEM structure scales elegantly for large design systems.

When in doubt, **favor readability and predictability over cleverness.**

---

## 🧪 Running Stylelint

```bash
npx stylelint "src/**/*.css"
```

To automatically fix minor issues:

```bash
npx stylelint "src/**/*.css" --fix
```

---

## ✅ Summary

| Category         | Goal                             |
| ---------------- | -------------------------------- |
| **Naming**       | Enforce kebab-case, prefixed BEM |
| **Specificity**  | Stay under `0,3,0`, no IDs       |
| **Declarations** | No `!important`, no redundancy   |
| **Organization** | Alphabetical property order      |
| **Formatting**   | Prettier handles whitespace      |
