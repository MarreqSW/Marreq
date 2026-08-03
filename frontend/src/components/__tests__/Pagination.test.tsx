import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Pagination } from '../table/Pagination';

describe('Pagination', () => {
  it('disables previous on the first page and next on the last page', () => {
    const onPageChange = vi.fn();
    const { rerender } = render(
      <Pagination page={1} pageCount={5} onPageChange={onPageChange} />,
    );
    const buttons = screen.getAllByRole('button');
    expect(buttons[0]).toBeDisabled();

    rerender(<Pagination page={5} pageCount={5} onPageChange={onPageChange} />);
    const lastButtons = screen.getAllByRole('button');
    expect(lastButtons[lastButtons.length - 1]).toBeDisabled();
  });

  it('calls onPageChange when a page number is clicked', async () => {
    const user = userEvent.setup();
    const onPageChange = vi.fn();
    render(<Pagination page={1} pageCount={5} onPageChange={onPageChange} />);

    await user.click(screen.getByRole('button', { name: '3' }));
    expect(onPageChange).toHaveBeenCalledWith(3);
  });

  it('navigates with previous and next controls', async () => {
    const user = userEvent.setup();
    const onPageChange = vi.fn();
    render(<Pagination page={3} pageCount={5} onPageChange={onPageChange} />);

    const buttons = screen.getAllByRole('button');
    await user.click(buttons[0]!);
    expect(onPageChange).toHaveBeenCalledWith(2);

    await user.click(buttons[buttons.length - 1]!);
    expect(onPageChange).toHaveBeenCalledWith(4);
  });

  it('renders ellipsis for large page counts', () => {
    render(<Pagination page={10} pageCount={20} onPageChange={vi.fn()} />);
    expect(screen.getAllByText('…').length).toBeGreaterThanOrEqual(1);
  });
});
