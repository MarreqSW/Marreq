import { beforeEach, describe, expect, it } from 'vitest';
import { init } from '@pages/groups.js';

describe('Groups Page', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('filters group rows and shows the filtered empty state', () => {
    document.body.innerHTML = `
      <input id="group-search-input" type="search">
      <div id="group-list">
        <article data-group-row data-search-text="Flight Systems Owner Project"></article>
        <article data-group-row data-search-text="Payload Viewer"></article>
      </div>
      <p id="group-empty-state">You do not belong to any groups yet.</p>
      <p id="group-empty-filter" hidden>No groups match your search.</p>
    `;

    init();

    const input = document.getElementById('group-search-input');
    const rows = document.querySelectorAll('[data-group-row]');
    const filteredEmpty = document.getElementById('group-empty-filter');

    input.value = 'payload';
    input.dispatchEvent(new Event('input'));

    expect(rows[0].hidden).toBe(true);
    expect(rows[1].hidden).toBe(false);
    expect(filteredEmpty.hidden).toBe(true);

    input.value = 'no-match';
    input.dispatchEvent(new Event('input'));

    expect(rows[0].hidden).toBe(true);
    expect(rows[1].hidden).toBe(true);
    expect(filteredEmpty.hidden).toBe(false);
  });

  it('hides the initial empty state once filtering starts', () => {
    document.body.innerHTML = `
      <input id="group-search-input" type="search">
      <div id="group-list">
        <article data-group-row data-search-text="Flight Systems"></article>
      </div>
      <p id="group-empty-state">You do not belong to any groups yet.</p>
      <p id="group-empty-filter" hidden>No groups match your search.</p>
    `;

    init();

    const input = document.getElementById('group-search-input');
    const initialEmpty = document.getElementById('group-empty-state');

    input.value = 'flight';
    input.dispatchEvent(new Event('input'));

    expect(initialEmpty.hidden).toBe(true);
  });
});
