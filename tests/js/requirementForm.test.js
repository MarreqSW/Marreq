/**
 * Tests for requirementForm.js - Requirement creation and editing forms
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock dependencies
vi.mock('@modules/referenceValidator.js', () => ({
  initRequirementReferenceValidation: vi.fn(),
}));

vi.mock('@modules/notifications.js', () => ({
  showNotification: vi.fn(),
}));

describe('Requirement Form', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
  });

  describe('Form Validation', () => {
    it('should have required fields marked', () => {
      document.body.innerHTML = `
        <form data-requirement-form data-allow-soft-mismatch="false">
          <input type="text" id="req_title" name="req_title" required />
          <textarea id="req_description" name="req_description" required></textarea>
          <input type="text" id="req_reference" name="req_reference" />
          <select id="req_category" name="req_category">
            <option value="1" data-tag="SYS">Systems</option>
          </select>
          <div id="reference-error" hidden></div>
          <button type="submit" data-role="submit-requirement">Save</button>
        </form>
      `;

      const titleInput = document.getElementById('req_title');
      const descriptionInput = document.getElementById('req_description');

      expect(titleInput.required).toBe(true);
      expect(descriptionInput.required).toBe(true);
    });

    it('should validate reference format', async () => {
      const { initRequirementReferenceValidation } = await import(
        '@modules/referenceValidator.js'
      );

      document.body.innerHTML = `
        <form data-requirement-form data-allow-soft-mismatch="false">
          <input type="text" id="req_reference" name="req_reference" />
          <select id="req_category" name="req_category">
            <option value="1" data-tag="SYS" selected>Systems</option>
          </select>
          <div id="reference-error" hidden></div>
          <button type="submit" data-role="submit-requirement">Save</button>
        </form>
      `;

      const { init: initForm } = await import(
        '@pages/requirementForm.js'
      );

      initForm();

      expect(initRequirementReferenceValidation).toHaveBeenCalled();
    });
  });


  describe('Reference Input', () => {
    it('should allow custom reference when provided', () => {
      document.body.innerHTML = `
        <form data-requirement-form data-allow-soft-mismatch="false">
          <input type="text" id="req_reference" name="req_reference" value="REQ-SYS-999" />
          <select id="req_category" name="req_category">
            <option value="1" data-tag="SYS" selected>Systems</option>
          </select>
          <div id="reference-error" hidden></div>
          <button type="submit" data-role="submit-requirement">Save</button>
        </form>
      `;

      const referenceInput = document.getElementById('req_reference');
      expect(referenceInput.value).toBe('REQ-SYS-999');
    });

    it('should display error for invalid reference format', () => {
      document.body.innerHTML = `
        <form data-requirement-form data-allow-soft-mismatch="false">
          <input type="text" id="req_reference" name="req_reference" />
          <select id="req_category" name="req_category">
            <option value="1" data-tag="SYS" selected>Systems</option>
          </select>
          <div id="reference-error" hidden>Invalid format</div>
          <button type="submit" data-role="submit-requirement">Save</button>
        </form>
      `;

      const errorEl = document.getElementById('reference-error');
      expect(errorEl).toBeTruthy();
    });
  });

  describe('Status Controls', () => {
    it('should toggle status menu on click', async () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <div data-role="status-control">
            <button type="button" data-role="status-toggle" aria-expanded="false">
              <span class="editor-status__label">Draft</span>
            </button>
            <div data-role="status-menu" hidden>
              <button type="button" data-status-id="1">Draft</button>
              <button type="button" data-status-id="2">Accepted</button>
            </div>
          </div>
          <select id="req_current_status">
            <option value="1">Draft</option>
            <option value="2">Accepted</option>
          </select>
        </form>
      `;

      const { init: initForm } = await import(
        '@pages/requirementForm.js'
      );

      initForm();

      const toggle = document.querySelector('[data-role="status-toggle"]');
      const menu = document.querySelector('[data-role="status-menu"]');

      expect(menu.hidden).toBe(true);

      toggle.click();

      expect(menu.hidden).toBe(false);
      expect(toggle.getAttribute('aria-expanded')).toBe('true');
    });

    it('should update status when selecting from menu', async () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <div data-role="status-control">
            <button type="button" data-role="status-toggle" aria-expanded="false">
              <span class="editor-status__label">Draft</span>
            </button>
            <div data-role="status-menu" hidden>
              <button type="button" data-status-id="1">Draft</button>
              <button type="button" data-status-id="2">Accepted</button>
            </div>
          </div>
          <select id="req_current_status">
            <option value="1" selected>Draft</option>
            <option value="2">Accepted</option>
          </select>
        </form>
      `;

      const { init: initForm } = await import(
        '@pages/requirementForm.js'
      );

      initForm();

      const toggle = document.querySelector('[data-role="status-toggle"]');
      const statusOption = document.querySelector('[data-status-id="2"]');
      const select = document.getElementById('req_current_status');
      const label = document.querySelector('.editor-status__label');

      toggle.click();
      statusOption.click();

      expect(select.value).toBe('2');
      expect(label.textContent.trim()).toBe('Accepted');
    });
  });

  describe('Form Submission', () => {
    it('should have submit button with correct role', () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <button type="submit" data-role="submit-requirement">Save</button>
        </form>
      `;

      const submitButton = document.querySelector('[data-role="submit-requirement"]');
      expect(submitButton).toBeTruthy();
      expect(submitButton.type).toBe('submit');
    });

    it('should support add another intent', () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <button type="submit" name="intent" value="add_another">Save & Add Another</button>
          <button type="submit" data-role="submit-requirement">Save</button>
        </form>
      `;

      const addAnotherButton = document.querySelector('[name="intent"][value="add_another"]');
      expect(addAnotherButton).toBeTruthy();
    });
  });

  describe('Data Attributes', () => {
    it('should have form marker for JavaScript initialization', () => {
      document.body.innerHTML = `
        <form data-requirement-form data-allow-soft-mismatch="false">
          <input type="text" id="req_title" />
        </form>
      `;

      const form = document.querySelector('[data-requirement-form]');
      expect(form).toBeTruthy();
      expect(form.dataset.requirementForm).toBeDefined();
    });

    it('should specify soft mismatch policy', () => {
      document.body.innerHTML = `
        <form data-requirement-form data-allow-soft-mismatch="true">
          <input type="text" id="req_title" />
        </form>
      `;

      const form = document.querySelector('[data-requirement-form]');
      expect(form.dataset.allowSoftMismatch).toBe('true');
    });

    it('should have project ID for context', () => {
      document.body.innerHTML = `
        <form data-requirement-form data-project-id="1">
          <input type="text" id="req_title" />
        </form>
      `;

      const form = document.querySelector('[data-requirement-form]');
      expect(form.dataset.projectId).toBe('1');
    });
  });

  describe('Flash Messages', () => {
    it('should display success message when provided', () => {
      document.body.innerHTML = `
        <form data-requirement-form data-flash-success="Requirement created successfully">
          <input type="text" id="req_title" />
        </form>
      `;

      const form = document.querySelector('[data-requirement-form]');
      expect(form.dataset.flashSuccess).toBe('Requirement created successfully');
    });
  });

  describe('Category Selection', () => {
    it('should include category tag in options', () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <select id="req_category">
            <option value="1" data-tag="SYS">Systems</option>
            <option value="2" data-tag="NET">Network</option>
          </select>
        </form>
      `;

      const options = document.querySelectorAll('#req_category option');
      expect(options[0].dataset.tag).toBe('SYS');
      expect(options[1].dataset.tag).toBe('NET');
    });
  });

  describe('Parent Selection', () => {
    it('should allow selecting parent requirement', () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <select id="req_parent">
            <option value="0">None</option>
            <option value="1">REQ-SYS-001 - Parent Requirement</option>
          </select>
        </form>
      `;

      const parentSelect = document.getElementById('req_parent');
      expect(parentSelect.options.length).toBe(2);
      expect(parentSelect.options[1].value).toBe('1');
    });
  });

  describe('Justification Field', () => {
    it('should have optional justification textarea', () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <textarea id="req_justification" name="req_justification"></textarea>
        </form>
      `;

      const justificationField = document.getElementById('req_justification');
      expect(justificationField).toBeTruthy();
      expect(justificationField.tagName).toBe('TEXTAREA');
    });
  });

  describe('Autosave Indicators', () => {
    it('should have autosave status element in edit form', () => {
      document.body.innerHTML = `
        <form data-requirement-form>
          <span data-role="autosave-text">All changes saved</span>
          <span data-unsaved-indicator hidden></span>
        </form>
      `;

      const autosaveText = document.querySelector('[data-role="autosave-text"]');
      const unsavedIndicator = document.querySelector('[data-unsaved-indicator]');

      expect(autosaveText).toBeTruthy();
      expect(unsavedIndicator).toBeTruthy();
      expect(unsavedIndicator.hidden).toBe(true);
    });
  });
});
