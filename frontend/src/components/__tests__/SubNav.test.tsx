import { render, screen } from '@testing-library/react';
import { MemoryRouter, Outlet, Route, Routes } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import SubNav from '../SubNav';

function renderAt(path: string) {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route
          path="/p/settings"
          element={
            <>
              <SubNav
                ariaLabel="Settings sections"
                items={[
                  { to: 'general', label: 'General', icon: 'tune' },
                  { to: 'catalog', label: 'Catalog' },
                  { to: 'secret', label: 'Hidden', hidden: true },
                ]}
              />
              <Outlet />
            </>
          }
        >
          <Route path="general" element={<p>general</p>} />
          <Route path="catalog/*" element={<p>catalog</p>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

describe('SubNav', () => {
  it('renders visible items as links relative to the route and marks the active one', () => {
    renderAt('/p/settings/catalog/categories');
    const nav = screen.getByRole('navigation', { name: 'Settings sections' });
    const links = Array.from(nav.querySelectorAll('a'));
    expect(links.map((a) => a.textContent)).toEqual(['tuneGeneral', 'Catalog']);
    expect(screen.getByRole('link', { name: /general/i })).toHaveAttribute('href', '/p/settings/general');
    expect(screen.getByRole('link', { name: 'Catalog' })).toHaveAttribute('aria-current', 'page');
    expect(screen.queryByRole('link', { name: 'Hidden' })).not.toBeInTheDocument();
  });
});
