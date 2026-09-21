import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import ClassicRequirementShowRedirect from '../ClassicRequirementShowRedirect';

describe('ClassicRequirementShowRedirect', () => {
  it('redirects classic show URLs to the SPA requirement view', () => {
    const { container } = render(
      <MemoryRouter initialEntries={['/space-project/requirements/show/42']}>
        <Routes>
          <Route path="/:projectSlug/requirements/show/:requirementId" element={<ClassicRequirementShowRedirect />} />
          <Route path="/:projectSlug/requirements/:requirementId" element={<div>current</div>} />
        </Routes>
      </MemoryRouter>,
    );
    expect(container.textContent).toBe('current');
  });

  it('redirects classic version URLs to the SPA snapshot route', () => {
    const { container } = render(
      <MemoryRouter initialEntries={['/space-project/requirements/show/42/version/101']}>
        <Routes>
          <Route
            path="/:projectSlug/requirements/show/:requirementId/version/:versionId"
            element={<ClassicRequirementShowRedirect />}
          />
          <Route
            path="/:projectSlug/requirements/:requirementId/versions/:versionId"
            element={<div>snapshot</div>}
          />
        </Routes>
      </MemoryRouter>,
    );
    expect(container.textContent).toBe('snapshot');
  });
});
