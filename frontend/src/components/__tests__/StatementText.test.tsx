import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import StatementText from '../StatementText';

describe('StatementText', () => {
  it('renders paragraphs, lists, emphasis and safe links', () => {
    render(
      <StatementText source={'The EPS shall support **three** modes:\n\n3. Off\n4. *Safe*\n\nSee [ICD](https://icd.test).'} />,
    );
    expect(screen.getByText('three').tagName).toBe('STRONG');
    const list = screen.getByRole('list');
    expect(list.tagName).toBe('OL');
    expect(list).toHaveAttribute('start', '3');
    expect(screen.getAllByRole('listitem')).toHaveLength(2);
    expect(screen.getByText('Safe').tagName).toBe('EM');
    const link = screen.getByRole('link', { name: 'ICD' });
    expect(link).toHaveAttribute('href', 'https://icd.test');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('never renders HTML or unsafe links from the source', () => {
    const { container } = render(
      <StatementText source={'<img src=x onerror=alert(1)> [x](javascript:alert(1))'} />,
    );
    expect(container.querySelector('img')).toBeNull();
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
    expect(container).toHaveTextContent('<img src=x onerror=alert(1)> [x](javascript:alert(1))');
  });

  it('shows the empty placeholder', () => {
    render(<StatementText source="   " />);
    expect(screen.getByText('—')).toBeInTheDocument();
  });
});
