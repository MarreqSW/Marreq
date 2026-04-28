/**
 * Tests for modules/sortTable.js
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { initTableSort, updateSortIndicators } from '@modules/sortTable.js';

describe('Sort Table', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  describe('initTableSort', () => {
    it('should return early if table is null', () => {
      expect(() => initTableSort(null, {})).not.toThrow();
    });

    it('should sort rows ascending on first click', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="name">Name</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Charlie</td></tr>
            <tr><td>Alice</td></tr>
            <tr><td>Bob</td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { name: 0 });

      const trigger = table.querySelector('.c-table-sort-trigger');
      trigger.click();

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].textContent.trim()).toBe('Alice');
      expect(rows[1].textContent.trim()).toBe('Bob');
      expect(rows[2].textContent.trim()).toBe('Charlie');
    });

    it('should sort rows descending on second click', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="name">Name</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Alice</td></tr>
            <tr><td>Bob</td></tr>
            <tr><td>Charlie</td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { name: 0 });

      const trigger = table.querySelector('.c-table-sort-trigger');
      trigger.click(); // First click: ascending
      trigger.click(); // Second click: descending

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].textContent.trim()).toBe('Charlie');
      expect(rows[1].textContent.trim()).toBe('Bob');
      expect(rows[2].textContent.trim()).toBe('Alice');
    });

    it('should reset to ascending when clicking different column', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="name">Name</button></th>
              <th><button class="c-table-sort-trigger" data-sort-key="age">Age</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Charlie</td><td>30</td></tr>
            <tr><td>Alice</td><td>25</td></tr>
            <tr><td>Bob</td><td>20</td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { name: 0, age: 1 });

      const nameTrigger = table.querySelector('[data-sort-key="name"]');
      const ageTrigger = table.querySelector('[data-sort-key="age"]');

      nameTrigger.click(); // Sort by name descending
      nameTrigger.click();
      ageTrigger.click(); // Switch to age (should be ascending)

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].querySelectorAll('td')[1].textContent.trim()).toBe('20');
      expect(rows[1].querySelectorAll('td')[1].textContent.trim()).toBe('25');
      expect(rows[2].querySelectorAll('td')[1].textContent.trim()).toBe('30');
    });

    it('should ignore clicks outside sort trigger', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="name">Name</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Charlie</td></tr>
            <tr><td>Alice</td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { name: 0 });

      const tbody = table.querySelector('tbody');
      tbody.click();

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].textContent.trim()).toBe('Charlie');
      expect(rows[1].textContent.trim()).toBe('Alice');
    });

    it('should ignore clicks on triggers outside table', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="name">Name</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Alice</td></tr>
          </tbody>
        </table>
        <button class="c-table-sort-trigger" data-sort-key="name">Outside</button>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { name: 0 });

      const outsideTrigger = document.querySelector('button[data-sort-key="name"]:not(table button)');
      outsideTrigger.click();

      // Should not have sorted
      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].textContent.trim()).toBe('Alice');
    });

    it('should ignore invalid sort keys', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="invalid">Invalid</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Charlie</td></tr>
            <tr><td>Alice</td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { name: 0 });

      const trigger = table.querySelector('.c-table-sort-trigger');
      trigger.click();

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].textContent.trim()).toBe('Charlie');
      expect(rows[1].textContent.trim()).toBe('Alice');
    });

    it('should use custom accessor function', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="value">Value</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td><input value="30"></td></tr>
            <tr><td><input value="10"></td></tr>
            <tr><td><input value="20"></td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { value: 0 }, {
        accessor: (cell) => cell.querySelector('input').value,
      });

      const trigger = table.querySelector('.c-table-sort-trigger');
      trigger.click();

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].querySelector('input').value).toBe('10');
      expect(rows[1].querySelector('input').value).toBe('20');
      expect(rows[2].querySelector('input').value).toBe('30');
    });

    it('should use default accessor for input elements', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="value">Value</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td><input value="30"></td></tr>
            <tr><td><input value="10"></td></tr>
            <tr><td><input value="20"></td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { value: 0 });

      const trigger = table.querySelector('.c-table-sort-trigger');
      trigger.click();

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].querySelector('input').value).toBe('10');
      expect(rows[1].querySelector('input').value).toBe('20');
      expect(rows[2].querySelector('input').value).toBe('30');
    });

    it('should use default accessor for select elements', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="value">Value</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td><select><option value="1" selected>Option C</option></select></td></tr>
            <tr><td><select><option value="2" selected>Option A</option></select></td></tr>
            <tr><td><select><option value="3" selected>Option B</option></select></td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { value: 0 });

      const trigger = table.querySelector('.c-table-sort-trigger');
      trigger.click();

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].querySelector('select option').textContent).toBe('Option A');
      expect(rows[1].querySelector('select option').textContent).toBe('Option B');
      expect(rows[2].querySelector('select option').textContent).toBe('Option C');
    });

    it('should use default accessor for span elements', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="value">Value</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td><span>30</span></td></tr>
            <tr><td><span>10</span></td></tr>
            <tr><td><span>20</span></td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { value: 0 });

      const trigger = table.querySelector('.c-table-sort-trigger');
      trigger.click();

      const rows = Array.from(table.querySelectorAll('tbody tr'));
      expect(rows[0].querySelector('span').textContent).toBe('10');
      expect(rows[1].querySelector('span').textContent).toBe('20');
      expect(rows[2].querySelector('span').textContent).toBe('30');
    });

    it('should prevent default on trigger click', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th><button class="c-table-sort-trigger" data-sort-key="name">Name</button></th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Alice</td></tr>
          </tbody>
        </table>
      `;

      const table = document.getElementById('testTable');
      initTableSort(table, { name: 0 });

      const trigger = table.querySelector('.c-table-sort-trigger');
      const clickEvent = new MouseEvent('click', { bubbles: true, cancelable: true });
      const preventDefaultSpy = vi.spyOn(clickEvent, 'preventDefault');
      trigger.dispatchEvent(clickEvent);

      expect(preventDefaultSpy).toHaveBeenCalled();
    });
  });

  describe('updateSortIndicators', () => {
    it('should reset all indicators', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th>
                <button class="c-table-sort-trigger" data-sort-key="name">
                  Name <span class="c-table-sort-indicator">↑</span>
                </button>
              </th>
              <th>
                <button class="c-table-sort-trigger" data-sort-key="age">
                  Age <span class="c-table-sort-indicator">↓</span>
                </button>
              </th>
            </tr>
          </thead>
        </table>
      `;

      const table = document.getElementById('testTable');
      updateSortIndicators(table, 'name', 'asc');

      const indicators = table.querySelectorAll('.c-table-sort-indicator');
      indicators.forEach((indicator) => {
        if (indicator.closest('[data-sort-key="name"]')) {
          expect(indicator.textContent).toBe('↑');
        } else {
          expect(indicator.textContent).toBe('↕');
        }
      });
    });

    it('should set active indicator to ascending', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th>
                <button class="c-table-sort-trigger" data-sort-key="name">
                  Name <span class="c-table-sort-indicator">↕</span>
                </button>
              </th>
            </tr>
          </thead>
        </table>
      `;

      const table = document.getElementById('testTable');
      updateSortIndicators(table, 'name', 'asc');

      const indicator = table.querySelector('.c-table-sort-indicator');
      expect(indicator.textContent).toBe('↑');
    });

    it('should set active indicator to descending', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th>
                <button class="c-table-sort-trigger" data-sort-key="name">
                  Name <span class="c-table-sort-indicator">↕</span>
                </button>
              </th>
            </tr>
          </thead>
        </table>
      `;

      const table = document.getElementById('testTable');
      updateSortIndicators(table, 'name', 'desc');

      const indicator = table.querySelector('.c-table-sort-indicator');
      expect(indicator.textContent).toBe('↓');
    });

    it('should handle missing indicator element', () => {
      document.body.innerHTML = `
        <table id="testTable">
          <thead>
            <tr>
              <th>
                <button class="c-table-sort-trigger" data-sort-key="name">Name</button>
              </th>
            </tr>
          </thead>
        </table>
      `;

      const table = document.getElementById('testTable');
      expect(() => updateSortIndicators(table, 'name', 'asc')).not.toThrow();
    });
  });
});
