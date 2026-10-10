import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import type { ProjectMember } from '@/api/types';
import CommentBody from '@/components/CommentBody';
import MentionTextarea from '@/components/MentionTextarea';

const member = (user_id: number, username: string, name: string): ProjectMember => ({
  user_id,
  username,
  name,
  role: 3,
  role_label: 'Author',
});
const members = [member(1, 'alice', 'Alice Smith'), member(2, 'bob', 'Bob Alvarez'), member(3, 'carol', 'Carol Diaz')];

function Harness({ list = members }: { list?: ProjectMember[] }) {
  const [value, setValue] = useState('');
  return <MentionTextarea value={value} onChange={setValue} members={list} ariaLabel="Add a comment" />;
}

describe('MentionTextarea', () => {
  it('suggests members after @ and inserts the chosen one with the keyboard', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const box = screen.getByRole('combobox', { name: 'Add a comment' });
    await user.type(box, 'Hi @al');
    const options = screen.getAllByRole('option');
    expect(options.map((o) => o.textContent)).toEqual(['Alice Smith@alice', 'Bob Alvarez@bob']);
    expect(options[0]).toHaveAttribute('aria-selected', 'true');

    await user.keyboard('{ArrowDown}{Enter}');
    expect(box).toHaveValue('Hi @bob ');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();

    // Typing straight on continues after the inserted mention.
    await user.keyboard('thanks');
    expect(box).toHaveValue('Hi @bob thanks');
  });

  it('inserts a member on click and closes on Escape', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const box = screen.getByRole('combobox', { name: 'Add a comment' });
    await user.type(box, '@c');
    await user.click(screen.getByRole('option', { name: /carol diaz/i }));
    expect(box).toHaveValue('@carol ');

    await user.type(box, '@a');
    expect(screen.getByRole('listbox')).toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    await user.keyboard('{Enter}');
    expect(box).toHaveValue('@carol @a\n');
  });

  it('is a plain textarea without members, and ignores e-mail addresses', async () => {
    const user = userEvent.setup();
    const { unmount } = render(<Harness list={[]} />);
    await user.type(screen.getByRole('textbox', { name: 'Add a comment' }), '@al');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    unmount();

    render(<Harness />);
    await user.type(screen.getByRole('combobox', { name: 'Add a comment' }), 'mail me@al');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });
});

describe('CommentBody', () => {
  it('highlights mentions of project members only', () => {
    render(<CommentBody body={'@Alice please check, cc @zed and bob@example.com'} members={members} />);
    const mention = screen.getByText('@Alice');
    expect(mention).toHaveAttribute('title', 'Alice Smith');
    expect(mention.tagName).toBe('SPAN');
    expect(screen.getByText(/please check, cc @zed and bob@example.com/)).toBeInTheDocument();
  });
});
