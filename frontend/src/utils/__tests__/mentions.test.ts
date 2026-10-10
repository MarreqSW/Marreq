import { describe, expect, it } from 'vitest';
import type { ProjectMember } from '@/api/types';
import { activeMention, splitMentions, suggestMembers } from '@/utils/mentions';

const usernames = (body: string) =>
  splitMentions(body).flatMap((p) => (p.kind === 'mention' ? [p.username] : []));

describe('splitMentions', () => {
  it('finds mentions at the start, middle and end, around punctuation', () => {
    expect(usernames('@bob ask (@carol), then @Dave.\n@erin!')).toEqual(['bob', 'carol', 'dave', 'erin']);
  });

  it('ignores e-mail addresses, double @ and names of the wrong length', () => {
    expect(usernames(`mail alice@example.com or @@bob @ab @${'a'.repeat(51)}`)).toEqual([]);
  });

  it('handles non-ASCII text like the server', () => {
    expect(usernames('Gràcies @jordi — ñ@bob @añé')).toEqual(['jordi']);
  });

  it('keeps the surrounding text intact', () => {
    expect(splitMentions('hi @bob!')).toEqual([
      { kind: 'text', text: 'hi ' },
      { kind: 'mention', text: '@bob', username: 'bob' },
      { kind: 'text', text: '!' },
    ]);
  });
});

describe('activeMention', () => {
  it('returns the query typed after @', () => {
    expect(activeMention('ask @al', 7)).toEqual({ start: 4, query: 'al' });
    expect(activeMention('@', 1)).toEqual({ start: 0, query: '' });
  });

  it('is null outside a mention or inside an e-mail address', () => {
    expect(activeMention('ask al', 6)).toBeNull();
    expect(activeMention('me@ex', 5)).toBeNull();
    expect(activeMention('@bob done', 9)).toBeNull();
  });
});

describe('suggestMembers', () => {
  const member = (user_id: number, username: string, name: string): ProjectMember => ({
    user_id,
    username,
    name,
    role: 3,
    role_label: 'Author',
  });
  const members = [member(1, 'alice', 'Alice Smith'), member(2, 'bob', 'Bob Alvarez'), member(3, 'carol', 'Carol')];

  it('ranks username prefixes before name matches', () => {
    expect(suggestMembers(members, 'al').map((m) => m.username)).toEqual(['alice', 'bob']);
  });

  it('lists everyone for an empty query', () => {
    expect(suggestMembers(members, '')).toHaveLength(3);
  });
});
