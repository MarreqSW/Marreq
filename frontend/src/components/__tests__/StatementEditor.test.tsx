import { useState } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import StatementEditor from '../StatementEditor';

function Harness({ initial = '' }: { initial?: string }) {
  const [value, setValue] = useState(initial);
  return (
    <>
      <StatementEditor id="stmt" ariaLabel="Statement" value={value} onChange={setValue} required />
      <output data-testid="value">{value}</output>
    </>
  );
}

describe('StatementEditor', () => {
  it('applies toolbar formatting to the selection', async () => {
    render(<Harness initial="The EPS shall boot" />);
    const user = userEvent.setup();
    const box = screen.getByLabelText('Statement') as HTMLTextAreaElement;
    box.setSelectionRange(14, 18);
    await user.click(screen.getByRole('button', { name: 'Bold' }));
    expect(screen.getByTestId('value')).toHaveTextContent('The EPS shall **boot**');
    expect(box.selectionStart).toBe(16);
    expect(box.selectionEnd).toBe(20);
  });

  it('supports Ctrl+I and numbered lists', async () => {
    render(<Harness initial={'Modes:\nOff\nSafe'} />);
    const user = userEvent.setup();
    const box = screen.getByLabelText('Statement') as HTMLTextAreaElement;
    box.setSelectionRange(7, box.value.length);
    await user.click(screen.getByRole('button', { name: 'Numbered list' }));
    expect(box.value).toBe('Modes:\n1. Off\n2. Safe');

    box.focus();
    box.setSelectionRange(0, 5);
    await user.keyboard('{Control>}i{/Control}');
    expect(box.value).toBe('*Modes*:\n1. Off\n2. Safe');
  });

  it('previews the formatted statement and disables the toolbar there', async () => {
    render(<Harness initial={'Use **bold**\n- item'} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole('tab', { name: 'Preview' }));
    const preview = screen.getByTestId('statement-preview');
    expect(preview.querySelector('strong')).toHaveTextContent('bold');
    expect(preview.querySelector('li')).toHaveTextContent('item');
    expect(screen.getByRole('button', { name: 'Bold' })).toBeDisabled();
    expect(screen.getByLabelText('Statement')).not.toBeVisible();

    await user.click(screen.getByRole('tab', { name: 'Write' }));
    expect(screen.getByLabelText('Statement')).toBeVisible();
  });

  it('counts bytes against the limit', () => {
    render(<Harness initial={'é'.repeat(1001)} />);
    expect(screen.getByText('2002 / 2000 bytes')).toBeInTheDocument();
  });
});
