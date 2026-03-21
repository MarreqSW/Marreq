/**
 * Tests for modules/diffModal.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { initDiffModal } from '@modules/diffModal.js';

describe('Diff Modal', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    window.bootstrap = {
      Modal: vi.fn(function(element) {
        this.element = element;
        this.show = vi.fn();
        this.hide = vi.fn();
        return this;
      }),
    };
  });

  it('should return early if modal element not found', () => {
    document.body.innerHTML = '<button data-action="show-changes">Show</button>';

    expect(() => initDiffModal({})).not.toThrow();
  });

  it('should return early if content element not found', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <button data-action="show-changes">Show</button>
    `;

    expect(() => initDiffModal({})).not.toThrow();
  });

  it('should return early if bootstrap not available', () => {
    window.bootstrap = undefined;
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes">Show</button>
    `;

    expect(() => initDiffModal({})).not.toThrow();
  });

  it('should initialize modal and attach click handler', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" data-old-values='{"key": "old"}' data-new-values='{"key": "new"}'>Show</button>
    `;

    initDiffModal({});

    expect(window.bootstrap.Modal).toHaveBeenCalled();
  });

  it('should show modal when trigger is clicked', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" data-old-values='{"key": "old"}' data-new-values='{"key": "new"}'>Show</button>
    `;

    initDiffModal({});

    const modalInstance = window.bootstrap.Modal.mock.results[0].value;
    const trigger = document.querySelector('[data-action="show-changes"]');
    trigger.click();

    expect(modalInstance.show).toHaveBeenCalled();
  });

  it('should generate JSON diff for JSON values', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" 
              data-old-values='{"name": "John", "age": 30}' 
              data-new-values='{"name": "Jane", "age": 30}'>Show</button>
    `;

    initDiffModal({});

    const trigger = document.querySelector('[data-action="show-changes"]');
    trigger.click();

    const content = document.getElementById('changesContent');
    expect(content.innerHTML).toContain('Old Values');
    expect(content.innerHTML).toContain('New Values');
    expect(content.innerHTML).toContain('"name": "John"');
    expect(content.innerHTML).toContain('"name": "Jane"');
  });

  it('should generate line diff for non-JSON values', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" 
              data-old-values="Line 1\nLine 2" 
              data-new-values="Line 1\nLine 3">Show</button>
    `;

    initDiffModal({});

    const trigger = document.querySelector('[data-action="show-changes"]');
    trigger.click();

    const content = document.getElementById('changesContent');
    expect(content.innerHTML).toContain('Old Values');
    expect(content.innerHTML).toContain('New Values');
  });

  it('should handle invalid JSON gracefully', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" 
              data-old-values="invalid json{" 
              data-new-values="also invalid}">Show</button>
    `;

    initDiffModal({});

    const trigger = document.querySelector('[data-action="show-changes"]');
    trigger.click();

    const content = document.getElementById('changesContent');
    expect(content.innerHTML).toContain('Old Values');
    expect(content.innerHTML).toContain('New Values');
  });

  it('should handle empty values', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" 
              data-old-values="" 
              data-new-values='{"key": "value"}'>Show</button>
    `;

    initDiffModal({});

    const trigger = document.querySelector('[data-action="show-changes"]');
    trigger.click();

    const content = document.getElementById('changesContent');
    expect(content.innerHTML).toContain('Old Values');
    expect(content.innerHTML).toContain('New Values');
  });

  it('should use custom selectors', () => {
    document.body.innerHTML = `
      <div id="customModal"></div>
      <div id="customContent"></div>
      <button class="custom-trigger" data-old-values='{"key": "old"}' data-new-values='{"key": "new"}'>Show</button>
    `;

    initDiffModal({
      triggerSelector: '.custom-trigger',
      modalSelector: '#customModal',
      contentSelector: '#customContent',
    });

    const trigger = document.querySelector('.custom-trigger');
    trigger.click();

    const modalInstance = window.bootstrap.Modal.mock.results[0].value;
    expect(modalInstance.show).toHaveBeenCalled();
  });

  it('should handle multiple triggers', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" data-old-values='{"a": 1}' data-new-values='{"a": 2}'>Show 1</button>
      <button data-action="show-changes" data-old-values='{"b": 1}' data-new-values='{"b": 2}'>Show 2</button>
    `;

    initDiffModal({});

    const triggers = document.querySelectorAll('[data-action="show-changes"]');
    triggers[0].click();

    const content = document.getElementById('changesContent');
    expect(content.innerHTML).toContain('"a": 1');
    expect(content.innerHTML).toContain('"a": 2');

    triggers[1].click();
    expect(content.innerHTML).toContain('"b": 1');
    expect(content.innerHTML).toContain('"b": 2');
  });

  it('should escape HTML in diff content', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" 
              data-old-values='{"key": "<script>alert(1)</script>"}' 
              data-new-values='{"key": "safe"}'></button>
    `;

    initDiffModal({});

    const trigger = document.querySelector('[data-action="show-changes"]');
    trigger.click();

    const content = document.getElementById('changesContent');
    // HTML should be escaped
    expect(content.innerHTML).toContain('&lt;script&gt;');
  });

  it('should highlight changed values in JSON diff', () => {
    document.body.innerHTML = `
      <div id="changesModal"></div>
      <div id="changesContent"></div>
      <button data-action="show-changes" 
              data-old-values='{"changed": "old", "unchanged": "same"}' 
              data-new-values='{"changed": "new", "unchanged": "same"}'></button>
    `;

    initDiffModal({});

    const trigger = document.querySelector('[data-action="show-changes"]');
    trigger.click();

    const content = document.getElementById('changesContent');
    // Changed values should have danger/success classes
    expect(content.innerHTML).toContain('text-danger');
    expect(content.innerHTML).toContain('text-success');
  });
});
