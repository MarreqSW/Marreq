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
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByRole('link', { name: /Table/i })).toHaveAttribute('aria-current', 'page');
    await user.click(screen.getByRole('link', { name: /List/i }));
    expect(screen.getByLabelText('location')).toHaveTextContent(
      '/demo/requirements?saved_view=12&view=list',
    );
    expect(screen.getByRole('link', { name: /List/i })).toHaveAttribute('aria-current', 'page');

    await user.click(screen.getByRole('link', { name: /Table/i }));
    expect(screen.getByLabelText('location')).toHaveTextContent('/demo/requirements?saved_view=12');
  });
});
