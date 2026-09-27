import { describe, expect, it } from 'vitest';
import { isSafeHref, parseInline, parseStatement, statementToPlainText } from '../statementMarkdown';

const t = (text: string) => ({ kind: 'text' as const, text });

// Mirrors the cases in marreq-core/src/rich_text.rs so both parsers stay in sync.
describe('parseStatement', () => {
  it('keeps plain text as one paragraph with line breaks', () => {
    expect(parseStatement('The system shall start.\nIt shall stop.')).toEqual([
      { kind: 'paragraph', inlines: [t('The system shall start.'), { kind: 'br' }, t('It shall stop.')] },
    ]);
    expect(parseStatement('  \n\n ')).toEqual([]);
  });

  it('parses lists and paragraphs', () => {
    const blocks = parseStatement('Power modes:\n1. Off\n2. **Safe**\n3. Nominal\n\n- a\n* b\n\nEnd');
    expect(blocks).toHaveLength(4);
    expect(blocks[1]).toEqual({
      kind: 'numbered',
      start: 1,
      items: [[t('Off')], [{ kind: 'strong', children: [t('Safe')] }], [t('Nominal')]],
    });
    expect(blocks[2]).toEqual({ kind: 'bullets', items: [[t('a')], [t('b')]] });
    expect(parseStatement('3. x\n4. y')[0]).toMatchObject({ kind: 'numbered', start: 3 });
  });
});

describe('parseInline', () => {
  it('parses the inline subset', () => {
    expect(parseInline('a **b** *c* _d_ `e*f` [g](https://x.test/p)')).toEqual([
      t('a '),
      { kind: 'strong', children: [t('b')] },
      t(' '),
      { kind: 'em', children: [t('c')] },
      t(' '),
      { kind: 'em', children: [t('d')] },
      t(' '),
      { kind: 'code', text: 'e*f' },
      t(' '),
      { kind: 'link', href: 'https://x.test/p', children: [t('g')] },
    ]);
  });

  it('leaves literal cases as text', () => {
    expect(parseInline('POWER_MODE_ONE')).toEqual([t('POWER_MODE_ONE')]);
    expect(parseInline('2 * 3 * 4')).toEqual([t('2 * 3 * 4')]);
    expect(parseInline('\\*not italic\\*')).toEqual([t('*not italic*')]);
    expect(parseInline('**unclosed')).toEqual([t('**unclosed')]);
  });

  it('never produces unsafe links', () => {
    for (const src of ['[x](javascript:alert(1))', '[x](data:text/html,hi)', '[](https://a.test)']) {
      expect(parseInline(src).some((n) => n.kind === 'link')).toBe(false);
    }
    expect(isSafeHref('mailto:ops@example.com')).toBe(true);
    expect(isSafeHref('https://a.test/ x')).toBe(false);
  });
});

describe('statementToPlainText', () => {
  it('drops markers and keeps list items on their own lines', () => {
    expect(
      statementToPlainText('The **system** shall:\n- boot\n- run `self-test`\n\nSee [spec](https://s.test).'),
    ).toBe('The system shall:\n• boot\n• run self-test\nSee spec.');
    expect(statementToPlainText('2. a\n3. b')).toBe('2. a\n3. b');
  });
});
