import { describe, expect, it } from 'vitest';
import { statusGlyph, statusSemanticGroup } from '../verificationStatusSemantic';

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
      className: 'text-emerald-400',
    });
    expect(statusGlyph('Failed', null)).toEqual({
      symbol: '✗',
      className: 'text-red-300',
    });
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
