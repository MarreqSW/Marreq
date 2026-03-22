/**
 * Tests for modules/referenceValidator.js
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  setSelectValues,
  initRequirementReferenceValidation,
  configureRequirementDeleteButton,
} from '@modules/referenceValidator.js';

describe('Reference Validator', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  describe('setSelectValues', () => {
    it('should set values for multiple select elements', () => {
      document.body.innerHTML = `
        <select id="select1">
          <option value="1">Option 1</option>
          <option value="2">Option 2</option>
        </select>
        <select id="select2">
          <option value="a">Option A</option>
          <option value="b">Option B</option>
        </select>
      `;

      setSelectValues({
        '#select1': '2',
        '#select2': 'b',
      });

      expect(document.getElementById('select1').value).toBe('2');
      expect(document.getElementById('select2').value).toBe('b');
    });

    it('should handle element selectors', () => {
      document.body.innerHTML = `
        <select id="select1">
          <option value="1">Option 1</option>
          <option value="2">Option 2</option>
        </select>
      `;

      setSelectValues({
        '#select1': '2',
      });

      expect(document.getElementById('select1').value).toBe('2');
    });

    it('should handle null values', () => {
      document.body.innerHTML = `
        <select id="select1">
          <option value="1">Option 1</option>
          <option value="">Empty</option>
        </select>
      `;

      setSelectValues({
        '#select1': null,
      });

      expect(document.getElementById('select1').value).toBe('');
    });

    it('should handle undefined values', () => {
      document.body.innerHTML = `
        <select id="select1">
          <option value="1">Option 1</option>
          <option value="">Empty</option>
        </select>
      `;

      setSelectValues({
        '#select1': undefined,
      });

      expect(document.getElementById('select1').value).toBe('');
    });

    it('should ignore non-existent selectors', () => {
      document.body.innerHTML = `
        <select id="select1">
          <option value="1">Option 1</option>
        </select>
      `;

      expect(() => setSelectValues({
        '#nonexistent': 'value',
        '#select1': '1',
      })).not.toThrow();

      expect(document.getElementById('select1').value).toBe('1');
    });

    it('should handle null selector', () => {
      expect(() => setSelectValues({
        [null]: 'value',
      })).not.toThrow();
    });
  });

  describe('initRequirementReferenceValidation', () => {
    it('should return early if reference input not found', () => {
      expect(() => initRequirementReferenceValidation({
        referenceSelector: '#nonexistent',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [],
      })).not.toThrow();
    });

    it('should return early if category select not found', () => {
      document.body.innerHTML = '<input id="reference">';
      expect(() => initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#nonexistent',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [],
      })).not.toThrow();
    });

    it('should validate reference format on input', () => {
      document.body.innerHTML = `
        <input id="reference">
        <select id="category">
          <option value="1">SYS</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [{ id: 1, tag: 'SYS' }],
      });

      const reference = document.getElementById('reference');
      const error = document.getElementById('error');
      const submit = document.getElementById('submit');

      reference.value = 'REQ-SYS-1';
      reference.dispatchEvent(new Event('input'));

      expect(error.hidden).toBe(true);
      expect(submit.disabled).toBe(false);
    });

    it('should show error for invalid format', () => {
      document.body.innerHTML = `
        <input id="reference">
        <select id="category">
          <option value="1">SYS</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [{ id: 1, tag: 'SYS' }],
      });

      const reference = document.getElementById('reference');
      const error = document.getElementById('error');
      const submit = document.getElementById('submit');

      reference.value = 'INVALID';
      reference.dispatchEvent(new Event('input'));

      expect(error.hidden).toBe(false);
      expect(error.textContent).toContain('REQ-SYS-');
      expect(submit.disabled).toBe(true);
    });

    it('should validate on category change', () => {
      document.body.innerHTML = `
        <input id="reference" value="REQ-SYS-1">
        <select id="category">
          <option value="1">SYS</option>
          <option value="2">PERF</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [
          { id: 1, tag: 'SYS' },
          { id: 2, tag: 'PERF' },
        ],
      });

      const category = document.getElementById('category');
      const error = document.getElementById('error');
      const submit = document.getElementById('submit');

      category.value = '2';
      category.dispatchEvent(new Event('change'));

      expect(error.hidden).toBe(false);
      expect(error.textContent).toContain('REQ-PERF-');
      expect(submit.disabled).toBe(true);
    });

    it('should allow empty reference', () => {
      document.body.innerHTML = `
        <input id="reference">
        <select id="category">
          <option value="1">SYS</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [{ id: 1, tag: 'SYS' }],
      });

      const reference = document.getElementById('reference');
      const error = document.getElementById('error');
      const submit = document.getElementById('submit');

      reference.value = '';
      reference.dispatchEvent(new Event('input'));

      expect(error.hidden).toBe(true);
      expect(submit.disabled).toBe(false);
    });

    it('should handle category not found', () => {
      document.body.innerHTML = `
        <input id="reference" value="REQ-SYS-1">
        <select id="category">
          <option value="999">Unknown</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [{ id: 1, tag: 'SYS' }],
      });

      const reference = document.getElementById('reference');
      const error = document.getElementById('error');
      const submit = document.getElementById('submit');

      reference.dispatchEvent(new Event('input'));

      expect(error.hidden).toBe(true);
      expect(submit.disabled).toBe(false);
    });

    it('should show warning with allowSoftMismatch', () => {
      document.body.innerHTML = `
        <input id="reference" value="REQ-OTHER-1">
        <select id="category">
          <option value="1">SYS</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [{ id: 1, tag: 'SYS' }],
        allowSoftMismatch: true,
      });

      const reference = document.getElementById('reference');
      const error = document.getElementById('error');
      const submit = document.getElementById('submit');

      reference.dispatchEvent(new Event('input'));

      expect(error.hidden).toBe(false);
      // With allowSoftMismatch, format errors still show but submit stays enabled
      expect(submit.disabled).toBe(false);
    });

    it('should use collect function for categories', () => {
      document.body.innerHTML = `
        <input id="reference" value="REQ-SYS-1">
        <select id="category">
          <option value="1">SYS</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      const collect = vi.fn(() => [{ id: 1, tag: 'SYS' }]);

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        collect,
      });

      const reference = document.getElementById('reference');
      reference.dispatchEvent(new Event('input'));

      expect(collect).toHaveBeenCalled();
    });

    it('should validate on initialization', () => {
      document.body.innerHTML = `
        <input id="reference" value="INVALID">
        <select id="category">
          <option value="1">SYS</option>
        </select>
        <div id="error" hidden></div>
        <button id="submit">Submit</button>
      `;

      initRequirementReferenceValidation({
        referenceSelector: '#reference',
        categorySelector: '#category',
        errorSelector: '#error',
        submitSelector: '#submit',
        categories: [{ id: 1, tag: 'SYS' }],
      });

      const error = document.getElementById('error');
      const submit = document.getElementById('submit');

      expect(error.hidden).toBe(false);
      expect(submit.disabled).toBe(true);
    });
  });

  describe('configureRequirementDeleteButton', () => {
    it('should return early if button not found', () => {
      expect(() => configureRequirementDeleteButton({
        buttonSelector: '#nonexistent',
        statusId: 1,
      })).not.toThrow();
    });

    it('should show button for admin', () => {
      document.body.innerHTML = '<button id="delete">Delete</button>';

      configureRequirementDeleteButton({
        buttonSelector: '#delete',
        statusId: 999,
        isAdmin: true,
      });

      const button = document.getElementById('delete');
      expect(button.style.display).toBe('inline-block');
    });

    it('should show button for allowed status', () => {
      document.body.innerHTML = '<button id="delete">Delete</button>';

      configureRequirementDeleteButton({
        buttonSelector: '#delete',
        statusId: 1,
        isAdmin: false,
        allowedStatuses: [1, 2],
      });

      const button = document.getElementById('delete');
      expect(button.style.display).toBe('inline-block');
    });

    it('should hide button for non-allowed status', () => {
      document.body.innerHTML = '<button id="delete">Delete</button>';

      configureRequirementDeleteButton({
        buttonSelector: '#delete',
        statusId: 3,
        isAdmin: false,
        allowedStatuses: [1, 2],
      });

      const button = document.getElementById('delete');
      expect(button.style.display).toBe('none');
    });

    it('should use default allowed statuses', () => {
      document.body.innerHTML = '<button id="delete">Delete</button>';

      configureRequirementDeleteButton({
        buttonSelector: '#delete',
        statusId: 1,
        isAdmin: false,
      });

      const button = document.getElementById('delete');
      expect(button.style.display).toBe('inline-block');
    });

    it('should handle status as string', () => {
      document.body.innerHTML = '<button id="delete">Delete</button>';

      configureRequirementDeleteButton({
        buttonSelector: '#delete',
        statusId: '1',
        isAdmin: false,
        allowedStatuses: [1, 2],
      });

      const button = document.getElementById('delete');
      expect(button.style.display).toBe('inline-block');
    });
  });
});
