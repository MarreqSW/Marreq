import { describe, expect, it } from 'vitest';
import {
  GROUP_TEXT_CLASS,
  STATUS_GROUP_OPTIONS,
  statusGlyph,
  statusSemanticGroup,
} from '../verificationStatusSemantic';

describe('statusSemanticGroup', () => {
  it('classifies fail and reject', () => {
    expect(statusSemanticGroup('Failed')).toBe('fail');
    expect(statusSemanticGroup('Rejected')).toBe('fail');
  });

  it('classifies pass and complete', () => {
    expect(statusSemanticGroup('Passed')).toBe('pass');
    expect(statusSemanticGroup('Complete')).toBe('pass');
    expect(statusSemanticGroup('Success')).toBe('pass');
    expect(statusSemanticGroup('ok')).toBe('pass');
  });

  it('classifies verified and accepted', () => {
    expect(statusSemanticGroup('Verified')).toBe('verified');
    expect(statusSemanticGroup('Accepted')).toBe('verified');
  });

  it('classifies pending and review', () => {
    expect(statusSemanticGroup('Pending')).toBe('pending');
    expect(statusSemanticGroup('In Review')).toBe('pending');
    expect(statusSemanticGroup('In Progress')).toBe('pending');
    expect(statusSemanticGroup('Blocked')).toBe('pending');
  });

  it('classifies draft', () => {
    expect(statusSemanticGroup('Draft')).toBe('draft');
  });

  it('prefers pass over review when both match', () => {
    expect(statusSemanticGroup('Passed review')).toBe('pass');
  });

  it('maps custom hex tag color to other', () => {
    expect(statusSemanticGroup('Custom', '#AABBCC')).toBe('other');
  });

  it('maps unknown titles without tag color to other', () => {
    expect(statusSemanticGroup('Not Run')).toBe('other');
  });
});

describe('statusGlyph', () => {
  it('returns semantic glyph classes for known groups', () => {
    expect(statusGlyph('Passed', null)).toEqual({
      symbol: '✓',
      className: 'text-emerald-700 dark:text-emerald-400',
    });
    expect(statusGlyph('Failed', null)).toEqual({
      symbol: '✗',
      className: 'text-red-700 dark:text-red-300',
    });
  });

  // Issue #363: every coloured group has a light-theme and a dark-theme shade.
  it('gives every coloured group a shade for each theme', () => {
    for (const group of ['pass', 'verified', 'pending', 'fail'] as const) {
      expect(GROUP_TEXT_CLASS[group]).toMatch(/^text-\w+-[6-8]00 dark:text-\w+-[2-4]00$/);
    }
    for (const option of STATUS_GROUP_OPTIONS) {
      expect(option.symbolClass).toBe(GROUP_TEXT_CLASS[option.id]);
    }
  });

  it('clears className for other statuses with a valid hex tag color', () => {
    expect(statusGlyph('Custom', '#AABBCC')).toEqual({
      symbol: '●',
      className: '',
    });
  });

  it('keeps muted class for other statuses without a hex tag color', () => {
    expect(statusGlyph('Not Run', null)).toEqual({
      symbol: '●',
      className: 'text-stitch-muted',
    });
  });
});
