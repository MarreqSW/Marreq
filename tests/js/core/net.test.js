/**
 * Tests for core/net.js - Network utility functions
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { jsonFetch, postJson, patchJson, deleteJson, formToJSON } from '@core/net.js';

describe('Network Utilities', () => {
  beforeEach(() => {
    global.fetch = vi.fn();
    vi.clearAllMocks();
  });

  describe('jsonFetch', () => {
    it('should make GET request by default', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true, "data": "test"}',
      });

      const result = await jsonFetch('/api/test');

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'GET',
          headers: expect.objectContaining({
            Accept: 'application/json',
          }),
          credentials: 'same-origin',
        }),
      );
      expect(result).toEqual({ success: true, data: 'test' });
    });

    it('should include custom headers', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await jsonFetch('/api/test', {
        headers: { 'X-Custom': 'value' },
      });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          headers: expect.objectContaining({
            Accept: 'application/json',
            'X-Custom': 'value',
          }),
        }),
      );
    });

    it('should send JSON body with POST', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await jsonFetch('/api/test', {
        method: 'POST',
        body: { key: 'value' },
      });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ key: 'value' }),
          headers: expect.objectContaining({
            'Content-Type': 'application/json',
          }),
        }),
      );
    });

    it('should send FormData without JSON stringification', async () => {
      const formData = new FormData();
      formData.append('key', 'value');

      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await jsonFetch('/api/test', {
        method: 'POST',
        body: formData,
      });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'POST',
          body: formData,
        }),
      );
      const callArgs = global.fetch.mock.calls[0][1];
      expect(callArgs.headers['Content-Type']).toBeUndefined();
    });

    it('should parse JSON response by default', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"key": "value"}',
      });

      const result = await jsonFetch('/api/test');
      expect(result).toEqual({ key: 'value' });
    });

    it('should return null for empty response when parse is true', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '',
      });

      const result = await jsonFetch('/api/test');
      expect(result).toBeNull();
    });

    it('should skip parsing when parse is false', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"key": "value"}',
      });

      const result = await jsonFetch('/api/test', { parse: false });
      expect(result).toBeNull();
    });

    it('should throw error on non-ok response', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
        text: async () => '{"message": "Not found"}',
      });

      await expect(jsonFetch('/api/test')).rejects.toThrow();
    });

    it('should throw error when payload.success is false', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": false, "message": "Error occurred"}',
      });

      await expect(jsonFetch('/api/test')).rejects.toThrow('Error occurred');
    });

    it('should include response and payload in error', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: async () => '{"message": "Server error"}',
      });

      try {
        await jsonFetch('/api/test');
        expect.fail('Should have thrown');
      } catch (error) {
        expect(error.response).toBeDefined();
        expect(error.payload).toEqual({ message: 'Server error' });
        expect(error.message).toContain('failed with status 500');
      }
    });

    it('should use custom credentials option', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await jsonFetch('/api/test', {
        credentials: 'include',
      });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          credentials: 'include',
        }),
      );
    });

    it('should handle null body', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await jsonFetch('/api/test', {
        method: 'POST',
        body: null,
      });

      const callArgs = global.fetch.mock.calls[0][1];
      expect(callArgs.body).toBeUndefined();
    });

    it('should handle undefined body', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await jsonFetch('/api/test', {
        method: 'POST',
        body: undefined,
      });

      const callArgs = global.fetch.mock.calls[0][1];
      expect(callArgs.body).toBeUndefined();
    });
  });

  describe('postJson', () => {
    it('should make POST request with JSON body', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await postJson('/api/test', { key: 'value' });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ key: 'value' }),
        }),
      );
    });

    it('should pass through additional options', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await postJson('/api/test', { key: 'value' }, { headers: { 'X-Custom': 'value' } });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({
            'X-Custom': 'value',
          }),
        }),
      );
    });
  });

  describe('patchJson', () => {
    it('should make PATCH request with JSON body', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await patchJson('/api/test', { key: 'value' });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'PATCH',
          body: JSON.stringify({ key: 'value' }),
        }),
      );
    });

    it('should pass through additional options', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      await patchJson('/api/test', { key: 'value' }, { headers: { 'X-Custom': 'value' } });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'PATCH',
          headers: expect.objectContaining({
            'X-Custom': 'value',
          }),
        }),
      );
    });
  });

  describe('deleteJson', () => {
    it('should make DELETE request', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '',
      });

      await deleteJson('/api/test');

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'DELETE',
        }),
      );
    });

    it('should not parse response by default', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      const result = await deleteJson('/api/test');
      expect(result).toBeNull();
    });

    it('should parse response when parse option is true', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '{"success": true}',
      });

      const result = await deleteJson('/api/test', { parse: true });
      expect(result).toEqual({ success: true });
    });

    it('should pass through additional options', async () => {
      global.fetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        text: async () => '',
      });

      await deleteJson('/api/test', { headers: { 'X-Custom': 'value' } });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'DELETE',
          headers: expect.objectContaining({
            'X-Custom': 'value',
          }),
        }),
      );
    });
  });

  describe('formToJSON', () => {
    it('should convert form to JSON object', () => {
      document.body.innerHTML = `
        <form id="testForm">
          <input name="name" value="John">
          <input name="email" value="john@example.com">
          <input name="age" value="30">
        </form>
      `;
      const form = document.getElementById('testForm');
      const result = formToJSON(form);

      expect(result).toEqual({
        name: 'John',
        email: 'john@example.com',
        age: '30',
      });
    });

    it('should handle empty form', () => {
      document.body.innerHTML = '<form id="testForm"></form>';
      const form = document.getElementById('testForm');
      const result = formToJSON(form);

      expect(result).toEqual({});
    });

    it('should handle form with no inputs', () => {
      document.body.innerHTML = '<form id="testForm"><div>No inputs</div></form>';
      const form = document.getElementById('testForm');
      const result = formToJSON(form);

      expect(result).toEqual({});
    });

    it('should handle multiple inputs with same name', () => {
      document.body.innerHTML = `
        <form id="testForm">
          <input name="tag" value="tag1">
          <input name="tag" value="tag2">
        </form>
      `;
      const form = document.getElementById('testForm');
      const result = formToJSON(form);

      // FormData.entries() returns the last value for duplicate keys
      expect(result.tag).toBe('tag2');
    });

    it('should handle select elements', () => {
      document.body.innerHTML = `
        <form id="testForm">
          <select name="category">
            <option value="1" selected>Category 1</option>
            <option value="2">Category 2</option>
          </select>
        </form>
      `;
      const form = document.getElementById('testForm');
      const result = formToJSON(form);

      expect(result).toEqual({ category: '1' });
    });

    it('should handle textarea elements', () => {
      document.body.innerHTML = `
        <form id="testForm">
          <textarea name="description">Test description</textarea>
        </form>
      `;
      const form = document.getElementById('testForm');
      const result = formToJSON(form);

      expect(result).toEqual({ description: 'Test description' });
    });
  });
});
