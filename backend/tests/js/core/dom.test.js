/**
 * Tests for core/dom.js - DOM utility functions
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { $, $$, on, dataSet, toArray } from '@core/dom.js';

describe('DOM Utilities', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  describe('$ (querySelector)', () => {
    it('should find a single element by selector', () => {
      document.body.innerHTML = '<div id="test">Hello</div>';
      const element = $('#test');
      expect(element).toBeTruthy();
      expect(element.id).toBe('test');
    });

    it('should return null when element not found', () => {
      document.body.innerHTML = '<div>Hello</div>';
      const element = $('#nonexistent');
      expect(element).toBeNull();
    });

    it('should search within a root element', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="item">Item 1</span>
        </div>
        <span class="item">Item 2</span>
      `;
      const container = $('#container');
      const item = $('.item', container);
      expect(item).toBeTruthy();
      expect(item.textContent).toBe('Item 1');
    });

    it('should work with class selectors', () => {
      document.body.innerHTML = '<div class="test-class">Test</div>';
      const element = $('.test-class');
      expect(element).toBeTruthy();
      expect(element.textContent).toBe('Test');
    });

    it('should work with attribute selectors', () => {
      document.body.innerHTML = '<div data-test="value">Test</div>';
      const element = $('[data-test="value"]');
      expect(element).toBeTruthy();
      expect(element.getAttribute('data-test')).toBe('value');
    });
  });

  describe('$$ (querySelectorAll)', () => {
    it('should find all matching elements', () => {
      document.body.innerHTML = `
        <div class="item">Item 1</div>
        <div class="item">Item 2</div>
        <div class="item">Item 3</div>
      `;
      const elements = $$('.item');
      expect(elements).toHaveLength(3);
      expect(elements[0].textContent).toBe('Item 1');
      expect(elements[1].textContent).toBe('Item 2');
      expect(elements[2].textContent).toBe('Item 3');
    });

    it('should return empty array when no matches found', () => {
      document.body.innerHTML = '<div>Hello</div>';
      const elements = $$('.nonexistent');
      expect(elements).toHaveLength(0);
      expect(Array.isArray(elements)).toBe(true);
    });

    it('should search within a root element', () => {
      document.body.innerHTML = `
        <div id="container">
          <span class="item">Item 1</span>
          <span class="item">Item 2</span>
        </div>
        <span class="item">Item 3</span>
      `;
      const container = $('#container');
      const items = $$('.item', container);
      expect(items).toHaveLength(2);
      expect(items[0].textContent).toBe('Item 1');
      expect(items[1].textContent).toBe('Item 2');
    });

    it('should return an array (not NodeList)', () => {
      document.body.innerHTML = '<div class="item">Item</div>';
      const elements = $$('.item');
      expect(Array.isArray(elements)).toBe(true);
      expect(elements).toHaveProperty('map');
      expect(elements).toHaveProperty('filter');
    });
  });

  describe('on (event delegation)', () => {
    it('should attach event listener to root element', () => {
      document.body.innerHTML = '<div id="container"><button class="btn">Click</button></div>';
      const handler = vi.fn();
      const container = $('#container');
      
      on(container, 'click', '.btn', handler);
      
      const button = $('.btn');
      button.click();
      
      expect(handler).toHaveBeenCalledTimes(1);
    });

    it('should call handler with event and matched element', () => {
      document.body.innerHTML = '<div id="container"><button class="btn">Click</button></div>';
      const handler = vi.fn();
      const container = $('#container');
      
      on(container, 'click', '.btn', handler);
      
      const button = $('.btn');
      button.click();
      
      expect(handler).toHaveBeenCalled();
      const [event, matchedElement] = handler.mock.calls[0];
      expect(event).toBeInstanceOf(Event);
      expect(matchedElement).toBe(button);
    });

    it('should not trigger for non-matching elements', () => {
      document.body.innerHTML = '<div id="container"><span>Text</span><button class="btn">Click</button></div>';
      const handler = vi.fn();
      const container = $('#container');
      
      on(container, 'click', '.btn', handler);
      
      const span = $('span');
      span.click();
      
      expect(handler).not.toHaveBeenCalled();
    });

    it('should use document as default root', () => {
      document.body.innerHTML = '<button class="btn">Click</button>';
      const handler = vi.fn();
      
      on(null, 'click', '.btn', handler);
      
      const button = $('.btn');
      button.click();
      
      expect(handler).toHaveBeenCalledTimes(1);
    });

    it('should support event options', () => {
      document.body.innerHTML = '<div id="container"><button class="btn">Click</button></div>';
      const handler = vi.fn();
      const container = $('#container');
      
      on(container, 'click', '.btn', handler, { once: true });
      
      const button = $('.btn');
      button.click();
      button.click();
      
      expect(handler).toHaveBeenCalledTimes(1);
    });

    it('should only match elements within root', () => {
      document.body.innerHTML = `
        <div id="container">
          <button class="btn">Inside</button>
        </div>
        <button class="btn">Outside</button>
      `;
      const handler = vi.fn();
      const container = $('#container');
      
      on(container, 'click', '.btn', handler);
      
      const insideBtn = container.querySelector('.btn');
      const outsideBtn = document.querySelectorAll('.btn')[1];
      
      insideBtn.click();
      expect(handler).toHaveBeenCalledTimes(1);
      
      outsideBtn.click();
      expect(handler).toHaveBeenCalledTimes(1); // Still 1, not 2
    });
  });

  describe('dataSet', () => {
    it('should get data attribute value', () => {
      document.body.innerHTML = '<div data-test="value">Test</div>';
      const element = $('div');
      expect(dataSet(element, 'test')).toBe('value');
    });

    it('should return fallback when attribute not found', () => {
      document.body.innerHTML = '<div>Test</div>';
      const element = $('div');
      expect(dataSet(element, 'nonexistent', 'default')).toBe('default');
    });

    it('should return null as default fallback', () => {
      document.body.innerHTML = '<div>Test</div>';
      const element = $('div');
      expect(dataSet(element, 'nonexistent')).toBeNull();
    });

    it('should return fallback when element is null', () => {
      expect(dataSet(null, 'test', 'default')).toBe('default');
    });

    it('should return fallback when element is null and no fallback provided', () => {
      expect(dataSet(null, 'test')).toBeNull();
    });

    it('should handle elements without dataset property', () => {
      const mockElement = { dataset: undefined };
      expect(dataSet(mockElement, 'test', 'default')).toBe('default');
    });

    it('should handle undefined dataset values', () => {
      const mockElement = { dataset: { test: undefined } };
      expect(dataSet(mockElement, 'test', 'default')).toBe('default');
    });

    it('should handle camelCase data attributes', () => {
      document.body.innerHTML = '<div data-test-value="result">Test</div>';
      const element = $('div');
      expect(dataSet(element, 'testValue')).toBe('result');
    });
  });

  describe('toArray', () => {
    it('should return array as-is when already an array', () => {
      const arr = [1, 2, 3];
      const result = toArray(arr);
      expect(result).toBe(arr);
      expect(result).toEqual([1, 2, 3]);
    });

    it('should wrap single value in array', () => {
      expect(toArray(1)).toEqual([1]);
      expect(toArray('string')).toEqual(['string']);
      expect(toArray(null)).toEqual([null]);
      expect(toArray(undefined)).toEqual([undefined]);
    });

    it('should wrap object in array', () => {
      const obj = { a: 1 };
      const result = toArray(obj);
      expect(result).toEqual([obj]);
      expect(result[0]).toBe(obj);
    });

    it('should handle empty array', () => {
      const arr = [];
      const result = toArray(arr);
      expect(result).toBe(arr);
      expect(result).toEqual([]);
    });
  });
});
