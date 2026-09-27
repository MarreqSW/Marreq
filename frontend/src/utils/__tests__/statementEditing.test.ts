import { describe, expect, it } from 'vitest';
import { insertLink, toggleList, toggleWrap, utf8Length } from '../statementEditing';

describe('toggleWrap', () => {
  it('wraps the selection and unwraps it again', () => {
    const wrapped = toggleWrap('shall boot', 6, 10, '**');
    expect(wrapped).toEqual({ value: 'shall **boot**', selectionStart: 8, selectionEnd: 12 });
    expect(toggleWrap(wrapped.value, 8, 12, '**')).toEqual({
      value: 'shall boot',
      selectionStart: 6,
      selectionEnd: 10,
    });
  });

  it('inserts a selected placeholder without a selection', () => {
    expect(toggleWrap('a ', 2, 2, '*')).toEqual({ value: 'a *italic text*', selectionStart: 3, selectionEnd: 14 });
  });

  it('does not treat bold markers as italic', () => {
    expect(toggleWrap('**x**', 2, 3, '*').value).toBe('***x***');
  });
});

describe('toggleList', () => {
  it('numbers the selected lines and removes the numbering again', () => {
    const src = 'Modes:\nOff\nSafe\nNominal';
    const numbered = toggleList(src, 7, src.length, 'numbered');
    expect(numbered.value).toBe('Modes:\n1. Off\n2. Safe\n3. Nominal');
    expect(toggleList(numbered.value, 7, numbered.value.length, 'numbered').value).toBe(src);
  });

  it('switches numbered items to bullets', () => {
    expect(toggleList('1. a\n2. b', 0, 9, 'bullets').value).toBe('- a\n- b');
  });
});

describe('insertLink', () => {
  it('uses the selection as label and selects the URL', () => {
    const r = insertLink('See spec now', 4, 8);
    expect(r.value).toBe('See [spec](https://) now');
    expect(r.value.slice(r.selectionStart, r.selectionEnd)).toBe('https://');
  });
});

it('counts UTF-8 bytes', () => {
  expect(utf8Length('é€')).toBe(5);
});
