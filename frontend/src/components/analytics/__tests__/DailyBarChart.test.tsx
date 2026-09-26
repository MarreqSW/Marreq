import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import DailyBarChart, { formatDay } from '../DailyBarChart';

const data = [
  { day: '2026-09-01', count: 2 },
  { day: '2026-09-02', count: 0 },
  { day: '2026-09-03', count: 5 },
];

describe('DailyBarChart', () => {
  it('renders one labelled bar per day and the max gridline value', () => {
    render(<DailyBarChart data={data} label="Events per day" />);
    const bars = screen.getAllByTestId('daily-bar');
    expect(bars).toHaveLength(3);
    expect(bars[2]).toHaveAccessibleName(`${formatDay('2026-09-03')}: 5 events`);
    expect(bars[1]).toHaveAccessibleName(`${formatDay('2026-09-02')}: 0 events`);
    expect(screen.getByRole('group', { name: 'Events per day' })).toBeInTheDocument();
    expect(screen.getByText('5', { selector: 'span' })).toBeInTheDocument();
  });

  it('shows a tooltip on hover and focus', () => {
    render(<DailyBarChart data={data} label="Events per day" />);
    const [first] = screen.getAllByTestId('daily-bar');
    fireEvent.mouseEnter(first);
    expect(screen.getByRole('tooltip')).toHaveTextContent(formatDay('2026-09-01'));
    fireEvent.blur(first);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('offers the values as a table', () => {
    render(<DailyBarChart data={data} label="Events per day" />);
    const table = screen.getByRole('table');
    expect(within(table).getAllByRole('row')).toHaveLength(4);
    expect(within(table).getByText('2026-09-03')).toBeInTheDocument();
  });
});
