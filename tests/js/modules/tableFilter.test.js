/**
 * Tests for modules/tableFilter.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { initSearchFilter } from '@modules/tableFilter.js';

describe('Table Filter', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('should return early if input not found', () => {
    expect(() => initSearchFilter({
      inputSelector: '#nonexistent',
      itemSelector: '.item',
    })).not.toThrow();
  });

  it('should return early if list not found', () => {
    document.body.innerHTML = '<input id="search">';

    expect(() => initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
      listSelector: '#nonexistent',
    })).not.toThrow();
  });

  it('should return early if no items found', () => {
    document.body.innerHTML = '<input id="search">';

    expect(() => initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    })).not.toThrow();
  });

  it('should filter items by text content', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">Banana</div>
        <div class="item">Cherry</div>
      </div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    });

    const input = document.getElementById('search');
    input.value = 'an';
    input.dispatchEvent(new Event('input'));

    const items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(true); // Apple - doesn't contain 'an'
    expect(items[1].hidden).toBe(false); // Banana - contains 'an'
    expect(items[2].hidden).toBe(true); // Cherry - doesn't contain 'an'
  });

  it('should show all items when query is empty', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">Banana</div>
      </div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    });

    const input = document.getElementById('search');
    input.value = 'an';
    input.dispatchEvent(new Event('input'));

    let items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(true);
    expect(items[1].hidden).toBe(false);

    input.value = '';
    input.dispatchEvent(new Event('input'));

    items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(false);
    expect(items[1].hidden).toBe(false);
  });

  it('should be case insensitive', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">BANANA</div>
        <div class="item">Cherry</div>
      </div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    });

    const input = document.getElementById('search');
    input.value = 'APPLE';
    input.dispatchEvent(new Event('input'));

    const items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(false);
    expect(items[1].hidden).toBe(true);
    expect(items[2].hidden).toBe(true);
  });

  it('should trim query before filtering', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">Banana</div>
      </div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    });

    const input = document.getElementById('search');
    input.value = '  apple  ';
    input.dispatchEvent(new Event('input'));

    const items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(false);
    expect(items[1].hidden).toBe(true);
  });

  it('should use custom getText function', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item" data-search-text="red fruit">Apple</div>
        <div class="item" data-search-text="yellow fruit">Banana</div>
      </div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
      getText: (element) => element.dataset.searchText || '',
    });

    const input = document.getElementById('search');
    input.value = 'red';
    input.dispatchEvent(new Event('input'));

    const items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(false);
    expect(items[1].hidden).toBe(true);
  });

  it('should show empty notice when no items match', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">Banana</div>
      </div>
      <div id="empty" hidden>No results</div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
      emptySelector: '#empty',
    });

    const input = document.getElementById('search');
    const empty = document.getElementById('empty');

    input.value = 'xyz';
    input.dispatchEvent(new Event('input'));

    expect(empty.hidden).toBe(false);
  });

  it('should hide empty notice when items match', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">Banana</div>
      </div>
      <div id="empty" hidden>No results</div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
      emptySelector: '#empty',
    });

    const input = document.getElementById('search');
    const empty = document.getElementById('empty');

    input.value = 'xyz';
    input.dispatchEvent(new Event('input'));
    expect(empty.hidden).toBe(false);

    input.value = 'app';
    input.dispatchEvent(new Event('input'));
    expect(empty.hidden).toBe(true);
  });

  it('should work without empty selector', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
      </div>
    `;

    expect(() => initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    })).not.toThrow();
  });

  it('should use custom list selector', () => {
    document.body.innerHTML = `
      <input id="search">
      <div id="container">
        <div class="item">Apple</div>
      </div>
      <div class="item">Banana</div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
      listSelector: '#container',
    });

    const input = document.getElementById('search');
    input.value = 'app';
    input.dispatchEvent(new Event('input'));

    const items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(false); // Inside container
    expect(items[1].hidden).toBe(false); // Outside container, not filtered
  });

  it('should call onFilter callback', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">Banana</div>
      </div>
    `;

    const onFilter = vi.fn();
    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
      onFilter,
    });

    const input = document.getElementById('search');
    input.value = 'app';
    input.dispatchEvent(new Event('input'));

    expect(onFilter).toHaveBeenCalledWith({
      count: 1,
      query: 'app',
      items: expect.any(Array),
    });
  });

  it('should return filter function', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Apple</div>
        <div class="item">Banana</div>
      </div>
    `;

    const result = initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    });

    expect(result).toBeTruthy();
    expect(result.filter).toBeInstanceOf(Function);

    result.filter('ban');
    const items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(true);
    expect(items[1].hidden).toBe(false);
  });

  it('should handle special characters in query', () => {
    document.body.innerHTML = `
      <input id="search">
      <div>
        <div class="item">Test (1)</div>
        <div class="item">Test [2]</div>
      </div>
    `;

    initSearchFilter({
      inputSelector: '#search',
      itemSelector: '.item',
    });

    const input = document.getElementById('search');
    input.value = '(1)';
    input.dispatchEvent(new Event('input'));

    const items = document.querySelectorAll('.item');
    expect(items[0].hidden).toBe(false);
    expect(items[1].hidden).toBe(true);
  });
});
