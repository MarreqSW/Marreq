/**
 * Tests for modules/inlineEdit.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { enableInlineTextEditing, enableInlineChangeHandling } from '@modules/inlineEdit.js';

describe('Inline Edit', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  describe('enableInlineTextEditing', () => {
    it('should return early if container is null', () => {
      expect(() => enableInlineTextEditing(null, '.editable')).not.toThrow();
    });

    it('should create input on click', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      enableInlineTextEditing(document.getElementById('container'), '.editable', vi.fn());

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      expect(input).toBeTruthy();
      expect(input.value).toBe('Original Text');
    });

    it('should not create duplicate input if already editing', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      enableInlineTextEditing(document.getElementById('container'), '.editable', vi.fn());

      const span = document.querySelector('.editable');
      span.click();
      span.click(); // Click again

      const inputs = span.querySelectorAll('input');
      expect(inputs).toHaveLength(1);
    });

    it('should return early if data-field is missing', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-id="1">Original Text</span>
        </div>
      `;

      enableInlineTextEditing(document.getElementById('container'), '.editable', vi.fn());

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      expect(input).toBeFalsy();
    });

    it('should return early if data-id is missing', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title">Original Text</span>
        </div>
      `;

      enableInlineTextEditing(document.getElementById('container'), '.editable', vi.fn());

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      expect(input).toBeFalsy();
    });

    it('should call onCommit with new value on blur', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      const onCommit = vi.fn();
      enableInlineTextEditing(document.getElementById('container'), '.editable', onCommit);

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      input.value = 'New Text';
      input.dispatchEvent(new Event('blur'));

      expect(onCommit).toHaveBeenCalledWith({
        id: '1',
        field: 'title',
        value: 'New Text',
        revert: expect.any(Function),
      });
    });

    it('should not call onCommit if value unchanged', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      const onCommit = vi.fn();
      enableInlineTextEditing(document.getElementById('container'), '.editable', onCommit);

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      input.value = 'Original Text';
      input.dispatchEvent(new Event('blur'));

      expect(onCommit).not.toHaveBeenCalled();
    });

    it('should commit on Enter key', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      const onCommit = vi.fn();
      enableInlineTextEditing(document.getElementById('container'), '.editable', onCommit);

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      input.value = 'New Text';
      const enterEvent = new KeyboardEvent('keydown', { key: 'Enter' });
      input.dispatchEvent(enterEvent);

      expect(onCommit).toHaveBeenCalled();
    });

    it('should revert on Escape key', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      enableInlineTextEditing(document.getElementById('container'), '.editable', vi.fn());

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      input.value = 'Changed Text';
      const escapeEvent = new KeyboardEvent('keydown', { key: 'Escape' });
      input.dispatchEvent(escapeEvent);

      expect(span.textContent).toBe('Original Text');
      expect(span.querySelector('input')).toBeFalsy();
    });

    it('should focus and select input text', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      enableInlineTextEditing(document.getElementById('container'), '.editable', vi.fn());

      const span = document.querySelector('.editable');
      const focusSpy = vi.spyOn(HTMLInputElement.prototype, 'focus');
      const selectSpy = vi.spyOn(HTMLInputElement.prototype, 'select');

      span.click();

      expect(focusSpy).toHaveBeenCalled();
      expect(selectSpy).toHaveBeenCalled();
    });

    it('should handle revert function from onCommit', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      let commitCallback;
      const onCommit = vi.fn((data) => {
        commitCallback = data.revert;
      });

      enableInlineTextEditing(document.getElementById('container'), '.editable', onCommit);

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      input.value = 'New Text';
      input.dispatchEvent(new Event('blur'));

      commitCallback();
      expect(span.textContent).toBe('Original Text');
    });

    it('should trim value before committing', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="editable" data-field="title" data-id="1">Original Text</span>
        </div>
      `;

      const onCommit = vi.fn();
      enableInlineTextEditing(document.getElementById('container'), '.editable', onCommit);

      const span = document.querySelector('.editable');
      span.click();

      const input = span.querySelector('input');
      input.value = '  Trimmed Text  ';
      input.dispatchEvent(new Event('blur'));

      expect(onCommit).toHaveBeenCalledWith(
        expect.objectContaining({
          value: 'Trimmed Text',
        }),
      );
    });
  });

  describe('enableInlineChangeHandling', () => {
    it('should return early if container is null', () => {
      expect(() => enableInlineChangeHandling(null, '.editable', vi.fn())).not.toThrow();
    });

    it('should call onChange on change event', () => {
      document.body.innerHTML = `
        <div id="container">
          <select class="editable" data-field="status" data-id="1">
            <option value="1">Active</option>
            <option value="2">Inactive</option>
          </select>
        </div>
      `;

      const onChange = vi.fn();
      enableInlineChangeHandling(document.getElementById('container'), '.editable', onChange);

      const select = document.querySelector('.editable');
      select.value = '2';
      select.dispatchEvent(new Event('change'));

      expect(onChange).toHaveBeenCalledWith({
        id: '1',
        field: 'status',
        value: '2',
      });
    });

    it('should return early if data-field is missing', () => {
      document.body.innerHTML = `
        <div id="container">
          <select class="editable" data-id="1">
            <option value="1">Active</option>
          </select>
        </div>
      `;

      const onChange = vi.fn();
      enableInlineChangeHandling(document.getElementById('container'), '.editable', onChange);

      const select = document.querySelector('.editable');
      select.dispatchEvent(new Event('change'));

      expect(onChange).not.toHaveBeenCalled();
    });

    it('should return early if data-id is missing', () => {
      document.body.innerHTML = `
        <div id="container">
          <select class="editable" data-field="status">
            <option value="1">Active</option>
          </select>
        </div>
      `;

      const onChange = vi.fn();
      enableInlineChangeHandling(document.getElementById('container'), '.editable', onChange);

      const select = document.querySelector('.editable');
      select.dispatchEvent(new Event('change'));

      expect(onChange).not.toHaveBeenCalled();
    });

    it('should handle multiple elements', () => {
      document.body.innerHTML = `
        <div id="container">
          <select class="editable" data-field="status" data-id="1">
            <option value="1">Active</option>
            <option value="2">Inactive</option>
          </select>
          <select class="editable" data-field="category" data-id="2">
            <option value="1">Cat 1</option>
            <option value="2">Cat 2</option>
          </select>
        </div>
      `;

      const onChange = vi.fn();
      enableInlineChangeHandling(document.getElementById('container'), '.editable', onChange);

      const selects = document.querySelectorAll('.editable');
      selects[0].value = '2';
      selects[0].dispatchEvent(new Event('change'));

      selects[1].value = '2';
      selects[1].dispatchEvent(new Event('change'));

      expect(onChange).toHaveBeenCalledTimes(2);
      expect(onChange).toHaveBeenNthCalledWith(1, {
        id: '1',
        field: 'status',
        value: '2',
      });
      expect(onChange).toHaveBeenNthCalledWith(2, {
        id: '2',
        field: 'category',
        value: '2',
      });
    });
  });
});
