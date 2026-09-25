import { Outlet, Route, Routes, useLocation } from 'react-router-dom';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import RequirementsViewSwitcher from '../RequirementsViewSwitcher';

function LocationProbe() {
  const location = useLocation();
  return <output aria-label="location">{`${location.pathname}${location.search}`}</output>;
}

function Layout() {
  return (
    <>
      <Outlet context={{ basePath: '/demo', projectId: 5, globalSearch: '', setGlobalSearch: () => undefined }} />
      <LocationProbe />
    </>
  );
}

describe('RequirementsViewSwitcher', () => {
  it('preserves saved-view query state while switching Table and List', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/demo/requirements?saved_view=12']}>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/demo/requirements" element={<RequirementsViewSwitcher />} />
            <Route path="/demo/traceability" element={<RequirementsViewSwitcher />} />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    const table = screen.getByRole('link', { name: /Table/i });
    const list = screen.getByRole('link', { name: /List/i });
    const graph = screen.getByRole('link', { name: /Graph/i });

    expect(table).toHaveAttribute('aria-current', 'page');
    expect(list).not.toHaveAttribute('aria-current');
    expect(graph).not.toHaveAttribute('aria-current');

    await user.click(screen.getByRole('link', { name: /List/i }));
    expect(screen.getByLabelText('location')).toHaveTextContent(
      '/demo/requirements?saved_view=12&view=list',
    );
    expect(list).toHaveAttribute('aria-current', 'page');
    expect(table).not.toHaveAttribute('aria-current');
    expect(graph).not.toHaveAttribute('aria-current');

    await user.click(screen.getByRole('link', { name: /Table/i }));
    expect(screen.getByLabelText('location')).toHaveTextContent('/demo/requirements?saved_view=12');
    expect(table).toHaveAttribute('aria-current', 'page');
    expect(list).not.toHaveAttribute('aria-current');
    expect(graph).not.toHaveAttribute('aria-current');

    await user.click(graph);
    expect(screen.getByLabelText('location')).toHaveTextContent('/demo/traceability');
    expect(graph).toHaveAttribute('aria-current', 'page');
    expect(table).not.toHaveAttribute('aria-current');
    expect(list).not.toHaveAttribute('aria-current');
  });
});
