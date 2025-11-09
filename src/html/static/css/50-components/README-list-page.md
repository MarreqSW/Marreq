# List Page Component

## Overview

The `list-page.css` component provides a reusable layout pattern for pages that display searchable, filterable lists of items. This component was created to eliminate code duplication between the categories and applicability pages, which had nearly identical styling.

## Architecture Decision

Following the **DRY (Don't Repeat Yourself)** principle and **Component-Based Architecture**, we extracted common styling into a shared component rather than maintaining duplicate CSS for each page type.

### Benefits

1. **Single Source of Truth**: Changes to the list layout only need to be made in one place
2. **Consistency**: All list-based pages automatically share the same look and feel
3. **Maintainability**: Reduced codebase size and complexity
4. **Scalability**: New list pages can be added quickly by reusing this component
5. **Performance**: Less CSS to download and parse

## Structure

The component follows BEM (Block Element Modifier) naming conventions:

```
.reqman-list-page              # Container
├── .reqman-list-page__header  # Page header with title and actions
│   └── .reqman-list-page__title
├── .reqman-list-page__toolbar # Search and filter controls
│   └── .reqman-list-page__search
└── .reqman-list-page__list    # List container
    └── .reqman-list-page__row # Individual list item
        ├── .reqman-list-page__row-main
        │   ├── .reqman-list-page__row-headline
        │   │   ├── .reqman-list-page__tag
        │   │   └── .reqman-list-page__row-title
        │   └── .reqman-list-page__row-description
        └── .reqman-list-page__row-meta
            ├── .reqman-list-page__row-id
            └── .reqman-list-page__row-actions
                └── .reqman-list-page__action-btn
```

## Usage

### HTML Template

```handlebars
<section class="reqman-list-page reqman-your-page">
  <div class="reqman-list-page__header">
    <h1 class="reqman-list-page__title">Your Title</h1>
    <a href="/new" class="btn btn-primary">New Item</a>
  </div>

  <div class="reqman-list-page__toolbar">
    <label for="search-input" class="u-sr-only">Search items</label>
    <input
      id="search-input"
      type="search"
      class="form-control reqman-list-page__search"
      placeholder="Search..."
    >
  </div>

  <div class="reqman-list-page__list" id="item-list">
    {{#each items}}
      <article class="reqman-list-page__row">
        <!-- Your item content -->
      </article>
    {{/each}}
  </div>
</section>
```

### Page-Specific CSS

Page-specific CSS files (e.g., `pages/categories.css`) should only contain:

1. Page-specific modifier classes (if needed)
2. Overrides for unique styling requirements
3. Documentation comments

```css
/**
 * Your Page
 * Inherits layout from list-page component.
 */

/* Page-specific overrides (if any) */
.reqman-your-page .reqman-list-page__tag {
  background: custom-gradient;
}
```

## Migration from Old Pattern

The refactoring changed class names from page-specific to component-based:

| Old Class                     | New Class                       |
|-------------------------------|---------------------------------|
| `.reqman-category-page`       | `.reqman-list-page`             |
| `.reqman-category-row`        | `.reqman-list-page__row`        |
| `.reqman-category-tag`        | `.reqman-list-page__tag`        |
| `.reqman-category-row__title` | `.reqman-list-page__row-title`  |
| (etc.)                        | (etc.)                          |

## Responsive Design

The component includes mobile-responsive styles that:
- Stack header elements vertically on small screens
- Make search input full-width
- Adjust row layout for touch-friendly interaction
- Optimize meta information display

Breakpoint: `768px`

## Accessibility

- Uses semantic HTML5 elements (`<section>`, `<article>`)
- Provides `.u-sr-only` labels for screen readers
- Maintains proper heading hierarchy
- Ensures keyboard navigation support through native elements

## Browser Support

The component uses modern CSS features with fallback values:
- CSS custom properties (with fallback values)
- Flexbox layout
- Grid (where applicable)

Compatible with all modern browsers and gracefully degrades in older browsers.

## Future Extensions

To extend this pattern for new list-based pages:

1. Use the `.reqman-list-page` base classes in your template
2. Add a page-specific class (e.g., `.reqman-statuses-page`)
3. Create a minimal page CSS file if custom styling is needed
4. Follow the established BEM naming pattern

## Related Files

- Component: `src/html/static/css/50-components/list-page.css`
- Uses this component:
  - `src/html/static/css/pages/categories.css`
  - `src/html/static/css/pages/applicability.css`
  - Templates: `templates/categories/`, `templates/applicability/`
